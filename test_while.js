function test_while() {
    let i = 0;
    while (i < 5) {
        i = i + 1;
    }
    return i;
}
let res = test_while();
ffi_console_println(res);
