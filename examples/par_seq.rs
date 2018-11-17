extern crate rayon;
#[macro_use(par, seq)]
extern crate shred;

use rayon::ThreadPoolBuilder;

use shred::{ParSeq, Read, Resources, System};

macro_rules! impl_sys {
    ($( $id:ident )*) => {
        $(
            impl ::shred::System for $id {
                type SystemData = ();
                fn run(&mut self, _: Self::SystemData) {
                    println!(stringify!($id));
                }
            }
        )*
    };
}

struct SysA;
struct SysB;
struct SysC;
struct SysD;
struct SysWithLifetime<'a>(&'a u8);
struct SysLocal(*const u8);

impl_sys!(SysA SysB SysC SysD SysLocal);

impl<'b> System for SysWithLifetime<'b> {
    type SystemData = Read<u64>;

    fn run(&mut self, nr: Read<u64>) {
        println!("SysWithLifetime, {}", *nr);
    }
}

fn main() {
    #![cfg_attr(rustfmt, rustfmt_skip)]

    let pool = ThreadPoolBuilder::new().build().expect("OS error");

    let mut res = Resources::new();
    let x = 5u8;

    let mut dispatcher = ParSeq::new(
        seq![
            par![
                SysA,
                SysWithLifetime(&x),
                seq![
                    SysC,
                    SysD,
                ],
            ],
            SysB,
            SysLocal(&x as *const u8),
        ],
        &pool,
    );

    dispatcher.setup(&mut res);
    dispatcher.dispatch(&mut res);

    // If we want to generate this graph from a `DispatcherBuilder`,
    // we can use `print_par_seq`:

    use shred::DispatcherBuilder;

    DispatcherBuilder::new()
        .with(SysA, "sys_a", &[])
        .with(SysWithLifetime(&x), "sys_lt", &[])
        .with(SysC, "sys_c", &[])
        .with(SysD, "sys_d", &["sys_c"])
        .with(SysB, "sys_b", &["sys_a", "sys_lt", "sys_c", "sys_d"])
        // doesn't work with `Dispatcher`
        // .with(SysLocal(&x as *const u8), "sys_local", &["sys_b"])
        .print_par_seq();

    // This prints:

    /*
seq![
    par![
        seq![
            sys_a,
        ],
        seq![
            sys_lt,
        ],
        seq![
            sys_c,
        ],
    ],
    par![
        seq![
            sys_d,
        ],
    ],
    par![
        seq![
            sys_b,
        ],
    ],
]
    */

    // This can now be pasted into a source file.
    // After replacing the system names with the actual systems, you can optimize it.
}
