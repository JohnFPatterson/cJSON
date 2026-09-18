fn main() {
    // Link libm for fabs / floating-point helpers used by the C-ABI port.
    println!("cargo:rustc-link-lib=m");
}
