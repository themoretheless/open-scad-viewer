 (func $6932 (param $0 i32) (param $1 i32) (param $2 i32) (param $3 f64) (param $4 f64) (param $5 f64) (param $6 f64)
  (local $7 i32)
  (local $8 i32)
  (local $9 i32)
  (local $10 i32)
  (local $11 i32)
  (local $12 i32)
  (local $13 i32)
  (local $14 i32)
  (local $15 f64)
  (local $16 f64)
  (local $17 f64)
  (local $18 f64)
  (local $19 f64)
  (local $20 f64)
  (local $21 f64)
  (local $22 f64)
  (local $23 v128)
  (global.set $global$0
   (local.tee $7
    (i32.sub
     (global.get $global$0)
     (i32.const 240)
    )
   )
  )
  (call $2266
   (i32.add
    (local.get $7)
    (i32.const 176)
   )
   (f64.const 1)
   (f64.const 1)
   (local.get $3)
   (local.get $4)
  )
  (block $label$1
   (if
    (i32.eq
     (i32.load offset=176
      (local.get $7)
     )
     (i32.const 1)
    )
    (block
     (local.set $1
      (i32.load offset=180
       (local.get $7)
      )
     )
     (v128.store offset=8 align=4
      (local.get $0)
      (v128.load offset=184 align=8
       (local.get $7)
      )
     )
     (i32.store offset=4
      (local.get $0)
      (local.get $1)
     )
     (i32.store
      (local.get $0)
      (i32.const -1)
     )
     (br $label$1)
    )
   )
   (call $856
    (i32.add
     (local.get $7)
     (i32.const 40)
    )
    (select
     (local.tee $15
      (select
       (f64.const 0)
       (local.tee $15
        (f64.load offset=184
         (local.get $7)
        )
       )
       (f64.ne
        (local.get $15)
        (local.get $15)
       )
      )
     )
     (f64.const 0)
     (f64.gt
      (local.get $15)
      (f64.const 0)
     )
    )
    (select
     (local.tee $15
      (select
       (f64.const 1)
       (local.tee $15
        (f64.load offset=192
         (local.get $7)
        )
       )
       (f64.ne
        (local.get $15)
        (local.get $15)
       )
      )
     )
     (f64.const 1)
     (f64.lt
      (local.get $15)
      (f64.const 1)
     )
    )
   )
   (if
    (i32.eq
     (i32.load offset=40
      (local.get $7)
     )
     (i32.const 1)
    )
    (block
     (local.set $1
      (i32.load offset=44
       (local.get $7)
      )
     )
     (v128.store offset=8 align=4
      (local.get $0)
      (v128.load offset=48 align=8
       (local.get $7)
      )
     )
     (i32.store offset=4
      (local.get $0)
      (local.get $1)
     )
     (i32.store
      (local.get $0)
      (i32.const -1)
     )
     (br $label$1)
    )
   )
   (local.set $19
    (f64.load offset=56
     (local.get $7)
    )
   )
   (local.set $20
    (f64.load offset=48
     (local.get $7)
    )
   )
   (call $6881
    (i32.add
     (local.get $7)
     (i32.const 4)
    )
    (local.get $1)
    (local.get $2)
   )
   (memory.copy
    (local.tee $1
     (call $3195
      (i32.const 8)
      (i32.const 64)
     )
    )
    (call $6875
     (local.tee $2
      (i32.load offset=8
       (local.get $7)
      )
     )
     (local.tee $9
      (i32.load offset=12
       (local.get $7)
      )
     )
     (i32.const 0)
     (i32.const 2203552)
    )
    (i32.const 64)
   )
   (i32.store offset=24
    (local.get $7)
    (i32.const 1)
   )
   (i32.store offset=20
    (local.get $7)
    (local.get $1)
   )
   (i32.store offset=16
    (local.get $7)
    (i32.const 1)
   )
   (local.set $1
    (call $3195
     (i32.const 8)
     (i32.const 64)
    )
   )
   (block $label$4
    (block $label$5
     (br_if $label$5
      (i32.eqz
       (local.get $9)
      )
     )
     (br_if $label$5
      (i32.eqz
       (local.tee $8
        (i32.add
         (i32.add
          (local.get $2)
          (i32.shl
           (local.get $9)
           (i32.const 6)
          )
         )
         (i32.const -64)
        )
       )
      )
     )
     (memory.copy
      (local.get $1)
      (local.get $8)
      (i32.const 64)
     )
     (i32.store offset=36
      (local.get $7)
      (i32.const 1)
     )
     (i32.store offset=32
      (local.get $7)
      (local.get $1)
     )
     (i32.store offset=28
      (local.get $7)
      (i32.const 1)
     )
     (local.set $1
      (local.get $9)
     )
     (loop $label$6
      (block $label$7
       (block $label$8
        (block $label$9
         (if
          (i32.ge_u
           (local.get $9)
           (i32.const 2)
          )
          (block
           (local.set $10
            (i32.sub
             (local.get $9)
             (i32.const 1)
            )
           )
           (local.set $8
            (i32.const 0)
           )
           (br $label$9)
          )
         )
         (call $6933
          (local.tee $11
           (i32.add
            (local.get $7)
            (i32.const 176)
           )
          )
          (local.tee $1
           (i32.load offset=32
            (local.get $7)
           )
          )
          (local.tee $10
           (i32.shr_u
            (local.tee $9
             (i32.load offset=36
              (local.get $7)
             )
            )
            (i32.const 1)
           )
          )
          (local.get $10)
          (i32.const 2236360)
         )
         (local.set $12
          (i32.load offset=180
           (local.get $7)
          )
         )
         (local.set $8
          (i32.load offset=176
           (local.get $7)
          )
         )
         (call $6933
          (local.get $11)
          (i32.sub
           (i32.add
            (local.get $1)
            (i32.shl
             (local.get $9)
             (i32.const 6)
            )
           )
           (local.tee $1
            (i32.shl
             (local.get $10)
             (i32.const 6)
            )
           )
          )
          (local.get $10)
          (local.get $10)
          (i32.const 2236376)
         )
         (local.set $9
          (i32.add
           (i32.add
            (local.get $1)
            (i32.load offset=176
             (local.get $7)
            )
           )
           (i32.const -64)
          )
         )
         (local.set $1
          (i32.const 0)
         )
         (local.set $13
          (i32.gt_u
           (local.tee $11
            (i32.load offset=180
             (local.get $7)
            )
           )
           (i32.sub
            (local.get $10)
            (i32.const 1)
           )
          )
         )
         (block $label$11
          (loop $label$12
           (br_if $label$11
            (i32.eqz
             (local.tee $14
              (i32.add
               (local.get $1)
               (local.get $10)
              )
             )
            )
           )
           (br_if $label$8
            (i32.eqz
             (i32.add
              (local.get $1)
              (local.get $12)
             )
            )
           )
           (if
            (local.get $13)
            (block
             (call $3651
              (local.get $8)
              (local.get $9)
              (i32.const 16)
             )
             (local.set $8
              (i32.sub
               (local.get $8)
               (i32.const -64)
              )
             )
             (local.set $9
              (i32.add
               (local.get $9)
               (i32.const -64)
              )
             )
             (local.set $1
              (i32.sub
               (local.get $1)
               (i32.const 1)
              )
             )
             (br $label$12)
            )
           )
          )
          (call $26
           (i32.sub
            (local.get $14)
            (i32.const 1)
           )
           (local.get $11)
           (i32.const 2081208)
          )
          (unreachable)
         )
         (i64.store align=4
          (local.get $0)
          (i64.load offset=16 align=4
           (local.get $7)
          )
         )
         (i32.store offset=184
          (local.get $7)
          (i32.load offset=24
           (local.get $7)
          )
         )
         (i64.store offset=188 align=4
          (local.get $7)
          (i64.load offset=28 align=4
           (local.get $7)
          )
         )
         (i64.store offset=8 align=4
          (local.get $0)
          (i64.load offset=184
           (local.get $7)
          )
         )
         (i32.store offset=196
          (local.get $7)
          (i32.load offset=36
           (local.get $7)
          )
         )
         (i64.store offset=16 align=4
          (local.get $0)
          (i64.load offset=192
           (local.get $7)
          )
         )
         (call $5030
          (i32.load offset=4
           (local.get $7)
          )
          (local.get $2)
         )
         (br $label$1)
        )
        (loop $label$14
         (br_if $label$7
          (i32.eq
           (local.get $8)
           (local.get $10)
          )
         )
         (memory.copy
          (i32.add
           (local.get $7)
           (i32.const 40)
          )
          (call $6875
           (local.get $2)
           (local.get $1)
           (local.get $8)
           (i32.const 2203616)
          )
          (i32.const 64)
         )
         (memory.copy
          (i32.add
           (local.get $7)
           (i32.const 176)
          )
          (call $6875
           (local.get $2)
           (local.get $1)
           (local.tee $12
            (i32.add
             (local.get $8)
             (i32.const 1)
            )
           )
           (i32.const 2203632)
          )
          (i32.const 64)
         )
         (local.set $2
          (i32.const 0)
         )
         (loop $label$15
          (if
           (i32.eq
            (local.get $2)
            (i32.const 64)
           )
           (block
            (call $856
             (i32.add
              (local.get $7)
              (i32.const 152)
             )
             (select
              (local.tee $15
               (select
                (local.get $5)
                (local.tee $15
                 (f64.load offset=48
                  (local.tee $11
                   (call $6875
                    (local.tee $2
                     (i32.load offset=8
                      (local.get $7)
                     )
                    )
                    (local.tee $1
                     (i32.load offset=12
                      (local.get $7)
                     )
                    )
                    (local.get $8)
                    (i32.const 2203648)
                   )
                  )
                 )
                )
                (f64.ne
                 (local.get $15)
                 (local.get $15)
                )
               )
              )
              (local.tee $16
               (select
                (local.get $15)
                (local.get $5)
                (f64.ne
                 (local.get $5)
                 (local.get $5)
                )
               )
              )
              (f64.gt
               (local.get $15)
               (local.get $16)
              )
             )
             (select
              (local.tee $15
               (select
                (local.get $6)
                (local.tee $15
                 (f64.load offset=56
                  (local.get $11)
                 )
                )
                (f64.ne
                 (local.get $15)
                 (local.get $15)
                )
               )
              )
              (local.tee $16
               (select
                (local.get $15)
                (local.get $6)
                (f64.ne
                 (local.get $6)
                 (local.get $6)
                )
               )
              )
              (f64.lt
               (local.get $15)
               (local.get $16)
              )
             )
            )
            (if
             (i32.ne
              (i32.load offset=152
               (local.get $7)
              )
              (i32.const 1)
             )
             (block
              (local.set $23
               (v128.load offset=160 align=8
                (local.get $7)
               )
              )
              (v128.store offset=48 align=8
               (call $6875
                (local.get $2)
                (local.get $1)
                (local.get $8)
                (i32.const 2203664)
               )
               (local.get $23)
              )
              (local.set $8
               (local.get $12)
              )
              (br $label$14)
             )
            )
            (local.set $1
             (i32.load offset=156
              (local.get $7)
             )
            )
            (v128.store offset=8 align=4
             (local.get $0)
             (v128.load offset=160 align=8
              (local.get $7)
             )
            )
            (br $label$4)
           )
          )
          (call $6885
           (i32.add
            (local.get $7)
            (i32.const 152)
           )
           (local.tee $17
            (f64.load
             (local.tee $1
              (i32.add
               (i32.add
                (local.get $7)
                (i32.const 40)
               )
               (local.get $2)
              )
             )
            )
           )
           (local.tee $18
            (f64.load
             (i32.add
              (local.get $1)
              (i32.const 8)
             )
            )
           )
           (local.get $20)
           (local.get $19)
          )
          (if
           (i32.eq
            (i32.load offset=152
             (local.get $7)
            )
            (i32.const 1)
           )
           (block
            (local.set $1
             (i32.load offset=156
              (local.get $7)
             )
            )
            (v128.store offset=8 align=4
             (local.get $0)
             (v128.load offset=160 align=8
              (local.get $7)
             )
            )
            (br $label$4)
           )
          )
          (local.set $21
           (f64.load offset=168
            (local.get $7)
           )
          )
          (local.set $22
           (f64.load offset=160
            (local.get $7)
           )
          )
          (call $6885
           (i32.add
            (local.get $7)
            (i32.const 152)
           )
           (local.tee $15
            (f64.load
             (local.tee $1
              (i32.add
               (i32.add
                (local.get $7)
                (i32.const 176)
               )
               (local.get $2)
              )
             )
            )
           )
           (local.tee $16
            (f64.load
             (i32.add
              (local.get $1)
              (i32.const 8)
             )
            )
           )
           (local.get $3)
           (local.get $4)
          )
          (if
           (i32.eq
            (i32.load offset=152
             (local.get $7)
            )
            (i32.const 1)
           )
           (block
            (local.set $1
             (i32.load offset=156
              (local.get $7)
             )
            )
            (v128.store offset=8 align=4
             (local.get $0)
             (v128.load offset=160 align=8
              (local.get $7)
             )
            )
            (br $label$4)
           )
          )
          (call $2267
           (i32.add
            (local.get $7)
            (i32.const 128)
           )
           (local.get $22)
           (local.get $21)
           (f64.load offset=160
            (local.get $7)
           )
           (f64.load offset=168
            (local.get $7)
           )
          )
          (if
           (i32.eq
            (i32.load offset=128
             (local.get $7)
            )
            (i32.const 1)
           )
           (block
            (local.set $1
             (i32.load offset=132
              (local.get $7)
             )
            )
            (v128.store offset=8 align=4
             (local.get $0)
             (v128.load offset=136 align=8
              (local.get $7)
             )
            )
            (br $label$4)
           )
          )
          (call $856
           (i32.add
            (local.get $7)
            (i32.const 104)
           )
           (select
            (local.tee $17
             (select
              (local.tee $15
               (select
                (local.tee $17
                 (select
                  (local.get $15)
                  (local.get $17)
                  (f64.ne
                   (local.get $17)
                   (local.get $17)
                  )
                 )
                )
                (local.tee $15
                 (select
                  (local.get $17)
                  (local.get $15)
                  (f64.ne
                   (local.get $15)
                   (local.get $15)
                  )
                 )
                )
                (f64.gt
                 (local.get $15)
                 (local.get $17)
                )
               )
              )
              (local.tee $17
               (f64.load offset=136
                (local.get $7)
               )
              )
              (f64.ne
               (local.get $17)
               (local.get $17)
              )
             )
            )
            (local.tee $15
             (select
              (local.get $17)
              (local.get $15)
              (f64.ne
               (local.get $15)
               (local.get $15)
              )
             )
            )
            (f64.lt
             (local.get $15)
             (local.get $17)
            )
           )
           (select
            (local.tee $16
             (select
              (local.tee $15
               (select
                (local.tee $15
                 (select
                  (local.get $16)
                  (local.get $18)
                  (f64.ne
                   (local.get $18)
                   (local.get $18)
                  )
                 )
                )
                (local.tee $16
                 (select
                  (local.get $15)
                  (local.get $16)
                  (f64.ne
                   (local.get $16)
                   (local.get $16)
                  )
                 )
                )
                (f64.gt
                 (local.get $15)
                 (local.get $16)
                )
               )
              )
              (local.tee $16
               (f64.load offset=144
                (local.get $7)
               )
              )
              (f64.ne
               (local.get $16)
               (local.get $16)
              )
             )
            )
            (local.tee $15
             (select
              (local.get $16)
              (local.get $15)
              (f64.ne
               (local.get $15)
               (local.get $15)
              )
             )
            )
            (f64.gt
             (local.get $15)
             (local.get $16)
            )
           )
          )
          (if
           (i32.eq
            (i32.load offset=104
             (local.get $7)
            )
            (i32.const 1)
           )
           (block
            (local.set $1
             (i32.load offset=108
              (local.get $7)
             )
            )
            (v128.store offset=8 align=4
             (local.get $0)
             (v128.load offset=112 align=8
              (local.get $7)
             )
            )
            (br $label$4)
           )
           (block
            (local.set $23
             (v128.load offset=112 align=8
              (local.get $7)
             )
            )
            (v128.store align=8
             (i32.add
              (call $6875
               (i32.load offset=8
                (local.get $7)
               )
               (i32.load offset=12
                (local.get $7)
               )
               (local.get $8)
               (i32.const 2203680)
              )
              (local.get $2)
             )
             (local.get $23)
            )
            (local.set $2
             (i32.add
              (local.get $2)
              (i32.const 16)
             )
            )
            (br $label$15)
           )
          )
         )
        )
       )
       (call $26
        (local.get $12)
        (local.get $12)
        (i32.const 2081192)
       )
       (unreachable)
      )
      (memory.copy
       (local.tee $8
        (i32.add
         (local.get $7)
         (i32.const 176)
        )
       )
       (call $6875
        (local.get $2)
        (local.get $1)
        (i32.const 0)
        (i32.const 2203584)
       )
       (i32.const 64)
      )
      (call $6934
       (i32.add
        (local.get $7)
        (i32.const 16)
       )
       (local.get $8)
      )
      (memory.copy
       (local.get $8)
       (call $6875
        (local.get $2)
        (local.get $1)
        (i32.sub
         (local.get $9)
         (i32.const 2)
        )
        (i32.const 2203600)
       )
       (i32.const 64)
      )
      (call $6934
       (i32.add
        (local.get $7)
        (i32.const 28)
       )
       (local.get $8)
      )
      (local.set $9
       (local.get $10)
      )
      (br $label$6)
     )
    )
    (call $55
     (i32.const 2203568)
    )
    (unreachable)
   )
   (i32.store offset=4
    (local.get $0)
    (local.get $1)
   )
   (i32.store
    (local.get $0)
    (i32.const -1)
   )
   (call $5030
    (i32.load offset=28
     (local.get $7)
    )
    (i32.load offset=32
     (local.get $7)
    )
   )
   (call $5030
    (i32.load offset=16
     (local.get $7)
    )
    (i32.load offset=20
     (local.get $7)
    )
   )
   (call $5030
    (i32.load offset=4
     (local.get $7)
    )
    (i32.load offset=8
     (local.get $7)
    )
   )
  )
  (global.set $global$0
   (i32.add
    (local.get $7)
    (i32.const 240)
   )
  )
 )