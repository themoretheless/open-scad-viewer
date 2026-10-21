 (func $6648 (param $0 f64) (result f64)
  (local $1 i64)
  (local $2 i64)
  (block $label$1
   (br_if $label$1
    (f64.ne
     (local.get $0)
     (local.get $0)
    )
   )
   (br_if $label$1
    (i64.eq
     (local.tee $1
      (i64.reinterpret_f64
       (local.get $0)
      )
     )
     (i64.const -4503599627370496)
    )
   )
   (local.set $0
    (f64.reinterpret_i64
     (block $label$2 (result i64)
      (drop
       (br_if $label$2
        (i64.const -9223372036854775807)
        (i64.eqz
         (local.tee $2
          (i64.and
           (local.get $1)
           (i64.const 9223372036854775807)
          )
         )
        )
       )
      )
      (drop
       (br_if $label$2
        (i64.add
         (local.get $1)
         (i64.const 1)
        )
        (i64.ne
         (local.get $1)
         (local.get $2)
        )
       )
      )
      (i64.sub
       (local.get $1)
       (i64.const 1)
      )
     )
    )
   )
  )
  (local.get $0)
 )