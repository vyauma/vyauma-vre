function test_array() {
    let arr = [1, 2, 3];
    arr[1] = 50;
    return arr[1] + arr[2];
}
let res = test_array();
ffi_console_println(res);
