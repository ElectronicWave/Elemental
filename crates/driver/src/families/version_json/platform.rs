use std::env::consts::{ARCH, OS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionJsonPlatform {
    os: String,
    arch: String,
}

impl VersionJsonPlatform {
    pub fn new(os: String, arch: String) -> Self {
        Self {
            os: normalize_os_name(os.as_str()),
            arch: normalize_arch_name(arch.as_str()),
        }
    }

    pub fn current() -> Self {
        Self::new(OS.to_owned(), ARCH.to_owned())
    }

    pub fn os(&self) -> &str {
        &self.os
    }

    pub fn arch(&self) -> &str {
        &self.arch
    }

    pub fn arch_bits(&self) -> &'static str {
        match self.arch.as_str() {
            "x86" | "arm32" | "mips" | "mipsel" | "ppc" | "ppcle" | "s390" | "riscv32"
            | "loongarch32" => "32",
            _ => "64",
        }
    }

    pub fn natives_dir_name(&self) -> String {
        format!("natives-{}-{}", self.os, self.arch)
    }

    /// Native descriptors used by the vanilla metadata, in HMCL lookup order.
    ///
    /// Each descriptor is `[variant-]os[-key]`, where variant is empty,
    /// `native`, or `natives`, and key is empty, the checked architecture
    /// name, or the architecture bit count.
    pub fn native_descriptors(&self) -> Vec<String> {
        let mut descriptors = Vec::new();
        for os_name in self.native_os_names() {
            for key in ["", self.arch.as_str(), self.arch_bits()] {
                for variant in ["", "native", "natives"] {
                    let mut descriptor = String::new();
                    if !variant.is_empty() {
                        descriptor.push_str(variant);
                        descriptor.push('-');
                    }
                    descriptor.push_str(os_name);
                    if !key.is_empty() {
                        descriptor.push('-');
                        descriptor.push_str(key);
                    }
                    descriptors.push(descriptor);
                }
            }
        }

        descriptors
    }

    pub fn native_os_names(&self) -> Vec<&str> {
        let mojang_name = self.mojang_os_name();
        if mojang_name == self.os.as_str() {
            vec![mojang_name]
        } else {
            vec![mojang_name, self.os.as_str()]
        }
    }

    fn mojang_os_name(&self) -> &str {
        match self.os.as_str() {
            "macos" => "osx",
            "freebsd" => "linux",
            os => os,
        }
    }
}

fn normalize_os_name(os: &str) -> String {
    match os.to_ascii_lowercase().as_str() {
        "osx" | "macosx" | "darwin" => "macos".to_owned(),
        normalized => normalized.to_owned(),
    }
}

fn normalize_arch_name(arch: &str) -> String {
    match arch.to_ascii_lowercase().as_str() {
        "x8664" | "x86-64" | "x86_64" | "amd64" | "ia32e" | "em64t" | "x64" | "intel64" => {
            "x86_64".to_owned()
        }
        "x8632" | "x86-32" | "x86_32" | "x86" | "i86pc" | "i386" | "i486" | "i586" | "i686"
        | "ia32" | "x32" => "x86".to_owned(),
        "arm64" | "aarch64" => "arm64".to_owned(),
        "arm" | "arm32" => "arm32".to_owned(),
        arch if arch.starts_with("armv7") => "arm32".to_owned(),
        arch if arch.starts_with("armv8") || arch.starts_with("armv9") => "arm64".to_owned(),
        arch => arch.to_owned(),
    }
}
