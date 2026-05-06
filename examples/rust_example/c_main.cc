#include <stdio.h>

extern "C" int call_from_c();

int main() {
  int val = call_from_c();
  printf("Value from Rust: %d\n", val);
  if (val == 42) {
    printf("SUCCESS!\n");
    return 0;
  }
  return 1;
}
