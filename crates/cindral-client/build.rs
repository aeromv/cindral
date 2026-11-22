// SPDX-License-Identifier: MIT OR Apache-2.0

//! Gives `cindral.exe` its icon and version details on Windows.

fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=cindral.rc");
        println!("cargo:rerun-if-changed=../../packaging/windows/cindral.ico");
        // Only the binary needs the icon; tests and benches link without it.
        embed_resource::compile_for("cindral.rc", ["cindral"], embed_resource::NONE)
            .manifest_optional()
            .expect("the icon resource compiles");
    }
}
