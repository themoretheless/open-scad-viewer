 (func $6649 (param $0 i32) (param $1 i32) (param $2 i32) (param $3 i32) (param $4 i32)
  (if
   (i32.eqz
    (local.get $1)
   )
   (block
    (call $4
     (local.get $0)
     (local.get $2)
     (i32.const 19)
     (local.get $3)
     (local.get $4)
    )
    (return)
   )
  )
  (i32.store
   (local.get $0)
   (i32.const -1)
  )
 )