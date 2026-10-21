 (func $857 (param $0 i32) (param $1 f64) (param $2 f64) (param $3 f64) (param $4 f64)
  (local $5 f64)
  (local $6 f64)
  (local $7 i32)
  (local $8 i32)
  (local $9 i32)
  (local $10 i32)
  (global.set $global$0
   (local.tee $8
    (i32.sub
     (global.get $global$0)
     (i32.const 32)
    )
   )
  )
  (local.set $5
   (f64.mul
    (local.get $2)
    (local.get $4)
   )
  )
  (local.set $4
   (f64.mul
    (local.get $1)
    (local.get $4)
   )
  )
  (call $6647
   (i32.add
    (local.get $8)
    (i32.const 12)
   )
   (i32.and
    (i32.eqz
     (i32.or
      (i32.or
       (i64.gt_u
        (i64.shl
         (i64.reinterpret_f64
          (local.tee $2
           (f64.mul
            (local.get $2)
            (local.get $3)
           )
          )
         )
         (i64.const 1)
        )
        (i64.const -9007199254740993)
       )
       (i64.ge_u
        (i64.shl
         (i64.reinterpret_f64
          (local.tee $1
           (f64.mul
            (local.get $1)
            (local.get $3)
           )
          )
         )
         (i64.const 1)
        )
        (i64.const -9007199254740992)
       )
      )
      (i64.ge_u
       (i64.shl
        (i64.reinterpret_f64
         (local.get $4)
        )
        (i64.const 1)
       )
       (i64.const -9007199254740992)
      )
     )
    )
    (i64.lt_u
     (i64.shl
      (i64.reinterpret_f64
       (local.get $5)
      )
      (i64.const 1)
     )
     (i64.const -9007199254740992)
    )
   )
   (i32.const 2115163)
   (i32.const 39)
  )
  (block $label$1
   (if
    (i32.ne
     (local.tee $7
      (i32.load offset=12
       (local.get $8)
      )
     )
     (i32.const -1)
    )
    (block
     (v128.store offset=8 align=8
      (local.get $0)
      (v128.load offset=16 align=4
       (local.get $8)
      )
     )
     (i32.store
      (local.get $0)
      (i32.const 1)
     )
     (i32.store offset=4
      (local.get $0)
      (local.get $7)
     )
     (br $label$1)
    )
   )
   (call $856
    (local.get $0)
    (call $6648
     (select
      (local.tee $3
       (select
        (local.tee $3
         (select
          (local.tee $3
           (select
            (local.tee $3
             (select
              (f64.const inf)
              (local.get $1)
              (local.tee $0
               (f64.ne
                (local.get $1)
                (local.get $1)
               )
              )
             )
            )
            (f64.const inf)
            (f64.lt
             (local.get $3)
             (f64.const inf)
            )
           )
          )
          (local.tee $6
           (select
            (local.get $3)
            (local.get $4)
            (local.tee $7
             (f64.ne
              (local.get $4)
              (local.get $4)
             )
            )
           )
          )
          (f64.lt
           (local.get $3)
           (local.get $6)
          )
         )
        )
        (local.tee $6
         (select
          (local.get $3)
          (local.get $2)
          (local.tee $9
           (f64.ne
            (local.get $2)
            (local.get $2)
           )
          )
         )
        )
        (f64.lt
         (local.get $3)
         (local.get $6)
        )
       )
      )
      (local.tee $6
       (select
        (local.get $3)
        (local.get $5)
        (local.tee $10
         (f64.ne
          (local.get $5)
          (local.get $5)
         )
        )
       )
      )
      (f64.lt
       (local.get $3)
       (local.get $6)
      )
     )
    )
    (call $5971
     (select
      (local.tee $1
       (select
        (local.tee $1
         (select
          (local.tee $1
           (select
            (local.tee $1
             (select
              (f64.const -inf)
              (local.get $1)
              (local.get $0)
             )
            )
            (f64.const -inf)
            (f64.gt
             (local.get $1)
             (f64.const -inf)
            )
           )
          )
          (local.tee $3
           (select
            (local.get $1)
            (local.get $4)
            (local.get $7)
           )
          )
          (f64.gt
           (local.get $1)
           (local.get $3)
          )
         )
        )
        (local.tee $2
         (select
          (local.get $1)
          (local.get $2)
          (local.get $9)
         )
        )
        (f64.gt
         (local.get $1)
         (local.get $2)
        )
       )
      )
      (local.tee $2
       (select
        (local.get $1)
        (local.get $5)
        (local.get $10)
       )
      )
      (f64.gt
       (local.get $1)
       (local.get $2)
      )
     )
    )
   )
  )
  (global.set $global$0
   (i32.add
    (local.get $8)
    (i32.const 32)
   )
  )
 )