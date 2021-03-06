#[lang = "panic_fmt"]
unsafe extern "C" fn panic_fmt(_: ::core::fmt::Arguments, _: &'static str, _: u32, _: u32) -> ! {
    ::abort()
}

#[lang = "start"]
extern "C" fn start<T>(main: fn() -> T, _argc: isize, _argv: *const *const u8) -> isize
    where
    T: Termination,
{
    main();

    0
}

#[lang = "termination"]
pub trait Termination {
    fn report(self) -> i32;
}

impl Termination for () {
    fn report(self) -> i32 {
        0
    }
}
