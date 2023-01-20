use bindings as c;

pub fn hrtick_start(cpu: i32, delay: u64) {
    unsafe {
        c::hrtick_start_cpu(cpu, delay);
    }
}
