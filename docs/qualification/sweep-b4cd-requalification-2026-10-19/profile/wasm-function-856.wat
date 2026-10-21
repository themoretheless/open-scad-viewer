 (func $856 (param $0 i32) (param $1 f64) (param $2 f64)
  (local $3 i32)
  (local $4 i32)
  (global.set $global$0
   (local.tee $3
    (i32.sub
     (global.get $global$0)
     (i32.const 32)
    )
   )
  )
  (call $6647
   (i32.add
    (local.get $3)
    (i32.const 12)
   )
   (i32.and
    (i32.and
     (i64.lt_u
      (i64.shl
       (i64.reinterpret_f64
        (local.get $2)
       )
       (i64.const 1)
      )
      (i64.const -9007199254740992)
     )
     (i64.lt_u
      (i64.shl
       (i64.reinterpret_f64
        (local.get $1)
       )
       (i64.const 1)
      )
      (i64.const -9007199254740992)
     )
    )
    (f64.le
     (local.get $1)
     (local.get $2)
    )
   )
   (i32.const 2115202)
   (i32.const 40)
  )
  (i32.store
   (local.get $0)
   (block $label$1 (result i32)
    (if
     (i32.ne
      (local.tee $4
       (i32.load offset=12
        (local.get $3)
       )
      )
      (i32.const -1)
     )
     (block
      (v128.store align=8
       (i32.add
        (local.get $0)
        (i32.const 8)
       )
       (v128.load offset=16 align=4
        (local.get $3)
       )
      )
      (i32.store offset=4
       (local.get $0)
       (local.get $4)
      )
      (br $label$1
       (i32.const 1)
      )
     )
    )
    (f64.store offset=16
     (local.get $0)
     (local.get $2)
    )
    (f64.store offset=8
     (local.get $0)
     (local.get $1)
    )
    (i32.const 0)
   )
  )
  (global.set $global$0
   (i32.add
    (local.get $3)
    (i32.const 32)
   )
  )
 )