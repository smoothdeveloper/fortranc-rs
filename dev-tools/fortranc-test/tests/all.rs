use fortranc_test::*;
#[test]
fn foo_here() {
  unsafe {
    assert_eq!(foo(), 108);
  }
}

#[test]
fn bar_here() {
  unsafe {
    assert_eq!(bar1(), 5);
    assert_eq!(bar2(), 6);
  }
}

