function test_object() {
    let obj = { "a": 10, "b": 20 };
    obj.b = 40;
    return obj.a + obj.b;
}
let res = test_object();
ffi_console_println(res);
