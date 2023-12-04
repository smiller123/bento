pub fn num_online_cpus() -> i32 {
    return num_cpus::get() as i32;
}
