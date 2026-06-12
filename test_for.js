function test_for() {
    let sum = 0;
    for (let i = 0; i < 4; i = i + 1) {
        sum = sum + i;
    }
    return sum;
}
let res = test_for();
ffi_console_println(res);
