function test_if(x) {
    if (x > 10) {
        return 1;
    } else {
        return 0;
    }
}
let res1 = test_if(20);
let res2 = test_if(5);
ffi_console_println(res1);
ffi_console_println(res2);
