module foo_module
implicit none
contains
#ifdef FOO
#if BAR == 1
function foo() bind(C) result(i)
  integer(4) :: i
  i = 108
end function
#endif
#endif

end module