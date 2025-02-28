function bar1() bind(C) result(i)
  use foo_module
  integer(4) :: i
  i = 5
end function
