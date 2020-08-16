use trapframe;

#[cfg(feature = "board_u540")]
#[path = "board/u540/mod.rs"]
pub mod board;
#[cfg(not(feature = "board_u540"))]
#[path = "board/virt/mod.rs"]
pub mod board;

pub mod compiler_rt;
pub mod consts;
pub mod cpu;
pub mod fp;
pub mod interrupt;
pub mod io;
pub mod memory;
pub mod paging;
pub mod rand;
pub mod sbi;
pub mod signal;
pub mod syscall;
pub mod timer;

use crate::memory::phys_to_virt;
use core::sync::atomic::{AtomicBool, Ordering};
use riscv::register::sie;

#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, device_tree_paddr: usize) -> ! {
    /*
    sbi::console_putchar('O' as usize);
    sbi::console_putchar('K' as usize);
    sbi::console_putchar('|' as usize);
    //sbi::console_putchar('\r\n' as usize);
     */
    info!("OK");

    let device_tree_vaddr = phys_to_virt(device_tree_paddr);

    unsafe {
        cpu::set_cpu_id(hartid);
    }

    if hartid != BOOT_HART_ID {
        while !AP_CAN_INIT.load(Ordering::Relaxed) {}
        others_main(hartid);
    }

    info!("BOOT_HART here");

    unsafe {
        memory::clear_bss();
    }

    info!("bss cleared");

    crate::logging::init();

    info!("init logging");

    unsafe {
        trapframe::init();
    }

    info!("init trapframe");

    memory::init(device_tree_vaddr);

    info!("init memory");

    println!(
        "Hello RISCV! in hart {}, device tree @ {:#x}",
        hartid, device_tree_vaddr
    );

    timer::init();

    info!("init timer");

    // TODO: init driver on u540
    #[cfg(not(any(feature = "board_u540")))]
    board::init(device_tree_vaddr);
    unsafe {
        board::init_external_interrupt();
    }

    info!("init driver");

    crate::process::init();

    info!("init process");

    AP_CAN_INIT.store(true, Ordering::Relaxed);
    crate::kmain();
}

fn others_main(hartid: usize) -> ! {
    unsafe {
        trapframe::init();
    }
    memory::init_other();
    timer::init();
    info!("Hello RISCV! in hart {}", hartid);
    crate::kmain();
}

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

#[cfg(not(feature = "board_u540"))]
const BOOT_HART_ID: usize = 0;
#[cfg(feature = "board_u540")]
const BOOT_HART_ID: usize = 1;

#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("boot/entry32.asm"));
#[cfg(target_arch = "riscv64")]
global_asm!(include_str!("boot/entry64.asm"));
