#include <mlx/c/fast.h>
#include <mlx/c/device.h>
#include <mlx/c/error.h>
#include <mlx/c/version.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(expr) do { if ((expr) != 0) { fprintf(stderr, "failure line %d\n", __LINE__); exit(3); } } while (0)
static void error(const char *message, void *unused) {
    (void)unused;
    fprintf(stderr, "MLX: %s\n", message);
}
static float decode(uint32_t bits) { float x; memcpy(&x, &bits, sizeof(x)); return x; }
static uint32_t encode(float x) { uint32_t bits; memcpy(&bits, &x, sizeof(bits)); return bits; }

static const char *source =
    "uint lane = thread_position_in_threadgroup.x;\n"
    "threadgroup float tile_a[64];\n"
    "threadgroup float tile_b[64];\n"
    "for (uint i=lane; i<64; i+=32) { tile_a[i]=as_type<float>(a[i]); tile_b[i]=as_type<float>(b[i]); }\n"
    "threadgroup_barrier(mem_flags::mem_threadgroup);\n"
    "simdgroup_float8x8 x, y;\n"
    "simdgroup_float8x8 z=make_filled_simdgroup_matrix<float,8,8>(0.0f);\n"
    "simdgroup_load(x,tile_a,8);\n"
    "simdgroup_load(y,tile_b,8);\n"
    "simdgroup_multiply_accumulate(z,x,y,z);\n"
    "simdgroup_store(z,out,8);\n";

static int probe(mlx_stream stream, mlx_fast_metal_kernel kernel,
                 const char *label, uint32_t a[64], uint32_t b[64]) {
    int input_shape[]={64}, output_shape[]={8,8};
    mlx_array left=mlx_array_new_data(a,input_shape,1,MLX_UINT32);
    mlx_array right=mlx_array_new_data(b,input_shape,1,MLX_UINT32);
    mlx_array operands[]={left,right};
    mlx_vector_array inputs=mlx_vector_array_new_data(operands,2);
    mlx_vector_array outputs=mlx_vector_array_new();
    mlx_fast_metal_kernel_config config=mlx_fast_metal_kernel_config_new();
    CHECK(mlx_fast_metal_kernel_config_add_output_arg(config,output_shape,2,MLX_FLOAT32));
    CHECK(mlx_fast_metal_kernel_config_set_grid(config,32,1,1));
    CHECK(mlx_fast_metal_kernel_config_set_thread_group(config,32,1,1));
    CHECK(mlx_fast_metal_kernel_apply(&outputs,kernel,inputs,config,stream));
    mlx_array output=mlx_array_new();
    CHECK(mlx_vector_array_get(&output,outputs,0));
    CHECK(mlx_array_eval(output));
    const float *actual=mlx_array_data_float32(output);
    if (!actual) exit(4);
    int mismatch=0; float first_expected=0;
    for (int row=0;row<8;row++) for(int col=0;col<8;col++) {
        double reference=0;
        for(int k=0;k<8;k++) reference+=(double)decode(a[row*8+k])*(double)decode(b[k*8+col]);
        float expected=(float)reference;
        if (row==0 && col==0) first_expected=expected;
        if (actual[row*8+col] != expected) mismatch++;
    }
    printf("%s mismatches=%d/64 first_actual=%.10g first_expected=%.10g\n",
           label,mismatch,actual[0],first_expected);
    mlx_array_free(output); mlx_vector_array_free(outputs); mlx_vector_array_free(inputs);
    mlx_fast_metal_kernel_config_free(config); mlx_array_free(left); mlx_array_free(right);
    return mismatch;
}

int main(void) {
    mlx_set_error_handler(error,NULL,NULL);
    mlx_device device=mlx_device_new_type(MLX_GPU,0);
    bool available=false; CHECK(mlx_device_is_available(&available,device));
    if(!available) { fputs("Metal GPU unavailable\n",stderr); return 2; }
    mlx_stream stream=mlx_stream_new_device(device);
    mlx_string version=mlx_string_new(); CHECK(mlx_version(&version));
    printf("MLX %s float SIMD-group 8x8 matrix probe\n",mlx_string_data(version));
    mlx_string_free(version);
    const char *input_names[]={"a","b"},*output_names[]={"out"};
    mlx_vector_string inputs=mlx_vector_string_new_data(input_names,2);
    mlx_vector_string outputs=mlx_vector_string_new_data(output_names,1);
    mlx_fast_metal_kernel kernel=mlx_fast_metal_kernel_new(
        "low_matmul_simdgroup_float_probe_v1",inputs,outputs,source,
        "#include <metal_simdgroup_matrix>\n",false,false);
    if(!kernel.ctx) return 4;
    uint32_t a[64],b[64]; int ordinary_failures=0;
    for(int replay=0;replay<3;replay++) {
        printf("replay=%d\n",replay);
        for(int i=0;i<64;i++) { a[i]=encode((float)((i*7)%17-8)/16); b[i]=encode((float)((i*11)%19-9)/16); }
        ordinary_failures+=probe(stream,kernel,"ordinary_dyadic",a,b);
        for(int i=0;i<64;i++) { a[i]=encode(0x1p-24f); b[i]=encode(65504.0f); }
        ordinary_failures+=probe(stream,kernel,"minimum_half_times_maximum_half",a,b);
        for(int i=0;i<64;i++) { a[i]=0x00010000u; b[i]=0x7f7f0000u; }
        probe(stream,kernel,"minimum_bf16_left_times_maximum_bf16",a,b);
        probe(stream,kernel,"minimum_bf16_right_times_maximum_bf16",b,a);
        for(int i=0;i<64;i++) { a[i]=0x007f0000u; b[i]=0x7f7f0000u; }
        probe(stream,kernel,"maximum_bf16_subnormal_times_maximum_bf16",a,b);
    }
    CHECK(mlx_synchronize(stream));
    mlx_fast_metal_kernel_free(kernel); mlx_vector_string_free(inputs); mlx_vector_string_free(outputs);
    mlx_stream_free(stream); mlx_device_free(device);
    printf("ordinary_failures=%d; BF16 subnormal cases are observations, not pass gates\n",ordinary_failures);
    return ordinary_failures ? 1 : 0;
}
