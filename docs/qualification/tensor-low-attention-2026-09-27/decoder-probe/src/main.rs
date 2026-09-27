use compute_core::{Binding, Kernel, gpu_compute::GpuContext, storage_u32, try_read_u32};
fn expected(bits:u32)->u32 {
    let sign=bits&0x8000;
    let exponent=(bits>>10)&31;
    let mantissa=bits&1023;
    if exponent==31 { return (sign<<16)|0x7f800000|(mantissa<<13); }
    if exponent==0 && mantissa==0 { return sign<<16; }
    let magnitude=if exponent==0 { f64::from(mantissa)*2f64.powi(-24) }
        else { (1.0+f64::from(mantissa)/1024.0)*2f64.powi(exponent as i32-15) };
    (if sign==0 { magnitude as f32 } else { -(magnitude as f32) }).to_bits()
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    let context=GpuContext::new().ok_or("actual GPU required for exhaustive unpack probe")?;
    println!("backend={} features={:?}",context.backend_label(),context.features);
    let neighbors=[0,0x8000,1,0x3c00,0x7bff,0x7c00,0x7e01,0xfc00,0x7c01];
    let mut packed=Vec::new(); let mut addresses=Vec::new(); let mut cases=Vec::new();
    for bits in 0u32..=65535 { for neighbor in neighbors { for parity in 0..2 {
        let address=(packed.len()*2+2+parity) as u32;
        packed.push(0xa55aa55a);
        packed.push(if parity==0 { bits|(neighbor<<16) }else{ neighbor|(bits<<16) });
        packed.push(0x5aa55aa5);
        addresses.push(address);cases.push((bits,neighbor,parity));
    }}}
    let input=storage_u32(&context.device,&context.queue,&packed);
    let indices=storage_u32(&context.device,&context.queue,&addresses);
    let output=storage_u32(&context.device,&context.queue,&vec![0xdeadbeef;cases.len()*4]);
    let kernel=Kernel::new(&context.device,"exhaustive f16 unpack probe",include_str!("../probe.wgsl"),"main",&[Binding::StorageRead,Binding::StorageRead,Binding::StorageReadWrite])?;
    for replay in 0..3 {
        kernel.dispatch(&context.device,&context.queue,&[&input,&indices,&output],cases.len() as u32);
        let actual=try_read_u32(&context.device,&context.queue,&output,cases.len()*4)?;
        let mut finite_mismatches=[0usize;4];let mut nan_mismatches=[0usize;4];let mut inf_mismatches=[0usize;4];
        let mut subnormal_mismatches=[0usize;4];
        for (i,&(bits,neighbor,parity)) in cases.iter().enumerate() {
            let want=expected(bits);let exp=bits&0x7c00;let mant=bits&1023;
            for method in 0..4 { let got=actual[i*4+method];
                if exp==0x7c00 && mant!=0 { nan_mismatches[method]+=usize::from(got&0x7fffffff<=0x7f800000); }
                else if exp==0x7c00 { inf_mismatches[method]+=usize::from(got!=want); }
                else if got!=want {
                    finite_mismatches[method]+=1;
                    if exp==0&&mant!=0 {subnormal_mismatches[method]+=1;}
                    if finite_mismatches[method]<=4 { println!("mismatch replay={replay} method={method} input={bits:04x} neighbor={neighbor:04x} parity={parity} expected={want:08x} actual={got:08x}"); }
                }
            }
        }
        println!("replay={replay} cases={} patterns=65536 neighbors=9 parities=2 methods=[packed_builtin,selected_builtin,guarded_builtin,integer] finite_mismatches={finite_mismatches:?} subnormal_mismatches={subnormal_mismatches:?} infinity_mismatches={inf_mismatches:?} nan_classification_mismatches={nan_mismatches:?}",cases.len());
        assert_eq!(finite_mismatches[2..],[0,0],"guarded candidate/integer must preserve all finite bits");
        assert_eq!(inf_mismatches[2..],[0,0]);assert_eq!(nan_mismatches[2..],[0,0]);
    }
    assert_eq!(try_read_u32(&context.device,&context.queue,&input,packed.len())?,packed);
    println!("PASS guarded exhaustive finite conversion inclsignedzero; inputs unchanged. Raw intrinsic outcomes are device observations, not a portable guarantee. No performance measurement.");
    Ok(())
}
