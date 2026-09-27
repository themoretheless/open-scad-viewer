use super::{CudaScalar, output};
use crate::{CudaError, CudaRuntime, CudaTensor, runtime::rank};
use gpu_compute::cuda::{CudaFunction, LaunchConfig, PushKernelArg, cudarc::driver::DeviceRepr};
use tensor_core::{Layout, ScanOptions, Shape};

struct ScanGeometry {
    rows: usize,
    length: usize,
    inner: usize,
    chunks: usize,
    metadata: Vec<u64>,
}
impl ScanGeometry {
    // Only used for a validated nonempty axis. Every product is bounded by
    // shape.numel(), whose byte count was checked before allocation/launch.
    fn new(layout: &Layout, axis: usize) -> Self {
        let shape = layout.shape();
        let length = shape.dims()[axis];
        let mut metadata: Vec<u64> = shape
            .dims()
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != axis)
            .map(|(_, &d)| d as u64)
            .collect();
        metadata.extend(
            layout
                .strides()
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != axis)
                .map(|(_, &s)| s as u64),
        );
        Self {
            rows: shape.numel() / length,
            length,
            inner: shape.dims()[axis + 1..].iter().product(),
            chunks: length.div_ceil(256),
            metadata,
        }
    }
}

impl CudaRuntime {
    pub(super) fn scan_typed<T: CudaScalar>(
        &self,
        input: &CudaTensor<T>,
        axis: usize,
        options: ScanOptions,
    ) -> Result<CudaTensor<T>, CudaError> {
        self.scan_loaded(input, axis, options, &self.indexing.scan[T::KIND], None)
    }

    // Only the first pass varies by source storage. Totals, recursive scans,
    // carries and the final output all use the accumulator type O.
    pub(crate) fn scan_loaded<I: DeviceRepr, O: CudaScalar>(
        &self,
        input: &CudaTensor<I>,
        axis: usize,
        options: ScanOptions,
        kernel: &CudaFunction,
        dtype: Option<u32>,
    ) -> Result<CudaTensor<O>, CudaError> {
        self.check(input)?;
        input.shape().validate_axes(&[axis])?;
        let mut out = self.zeros_typed(input.shape().clone())?;
        if input.shape().is_empty() {
            return Ok(out);
        }
        let geometry = ScanGeometry::new(&input.layout, axis);
        let mut totals =
            self.zeros_typed::<O>(Shape::new(vec![geometry.rows, geometry.chunks])?)?;
        let metadata = self.metadata(&geometry.metadata)?;
        let (rows, length, inner, chunks) = (
            geometry.rows as u64,
            geometry.length as u64,
            geometry.inner as u64,
            geometry.chunks as u64,
        );
        let row_rank = rank(input.shape())? - 1;
        let offset = input.layout.offset() as u64;
        let axis_stride = input.layout.strides()[axis] as u64;
        let inclusive = u32::from(options.inclusive);
        let reverse = u32::from(options.reverse);
        let groups = (geometry.rows * geometry.chunks)
            .min(self.device.multiprocessors as usize * 8)
            .max(1) as u32;
        // Each group owns a row/chunk, all barriers are uniform, and all
        // outputs are fresh. Padding lanes contribute zero without reading.
        unsafe {
            let mut launch = self.device.stream.launch_builder(kernel);
            launch
                .arg(input.storage.as_ref())
                .arg(output(&mut out))
                .arg(output(&mut totals))
                .arg(&metadata)
                .arg(&rows)
                .arg(&length)
                .arg(&inner)
                .arg(&chunks)
                .arg(&row_rank)
                .arg(&offset)
                .arg(&axis_stride)
                .arg(&inclusive)
                .arg(&reverse);
            if let Some(dtype) = dtype.as_ref() {
                launch.arg(dtype);
            }
            launch.launch(LaunchConfig {
                grid_dim: (groups, 1, 1),
                block_dim: (256, 1, 1),
                shared_mem_bytes: 0,
            })?;
        }
        if geometry.chunks > 1 {
            let offsets = self.scan_typed(
                &totals,
                1,
                ScanOptions {
                    inclusive: false,
                    reverse: false,
                },
            )?;
            let n = input.shape().numel() as u64;
            // The contiguous output is unique, and offsets have exactly one
            // entry for each original row/chunk, including the partial tail.
            unsafe {
                self.device
                    .stream
                    .launch_builder(&self.indexing.add_scan[O::KIND])
                    .arg(output(&mut out))
                    .arg(offsets.storage.as_ref())
                    .arg(&n)
                    .arg(&length)
                    .arg(&inner)
                    .arg(&chunks)
                    .arg(&reverse)
                    .launch(self.config(n as usize))?;
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_row_addressing_matches_strided_layout_for_every_axis() {
        let layout = Layout::contiguous(Shape::new(vec![3, 5, 517]).unwrap())
            .unwrap()
            .narrow(2, 3, 511)
            .unwrap()
            .permute(&[1, 2, 0])
            .unwrap();
        for axis in 0..3 {
            let g = ScanGeometry::new(&layout, axis);
            let row_dims = g.metadata[..2]
                .iter()
                .map(|&x| x as usize)
                .collect::<Vec<_>>();
            let row_strides = g.metadata[2..]
                .iter()
                .map(|&x| x as usize)
                .collect::<Vec<_>>();
            let rows =
                Layout::new(Shape::new(row_dims).unwrap(), row_strides, layout.offset()).unwrap();
            for row in 0..g.rows {
                for coordinate in 0..g.length {
                    let logical =
                        ((row / g.inner) * g.length + coordinate) * g.inner + row % g.inner;
                    assert_eq!(
                        rows.element_offset(row).unwrap() + coordinate * layout.strides()[axis],
                        layout.element_offset(logical).unwrap()
                    );
                }
            }
            assert_eq!(g.rows * g.length, layout.shape().numel());
            assert_eq!(g.chunks, g.length.div_ceil(256));
        }
    }
}
