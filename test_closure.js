// test_closure.js — Phase 11.3: Closures and Lexical Scoping

// Test 1: Counter factory — closure captures `count`
function makeCounter(start) {
  let count = start;
  let inc = () => {
    count = count + 1;
    return count;
  };
  return inc;
}

let counter = makeCounter(10);
let a = counter();   // 11
let b = counter();   // 12
ffi_console_println(a);  // Expected: 11
ffi_console_println(b);  // Expected: 12

// Test 2: Simple adder closure
function makeAdder(x) {
  return (y) => {
    return x + y;
  };
}

let add5 = makeAdder(5);
let result = add5(3);   // 8
ffi_console_println(result);  // Expected: 8
