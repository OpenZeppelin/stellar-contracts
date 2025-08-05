fn main() {
    let packages = vec![
        "stellar-shared-storage",
        "shared-storage-1",
        "shared-storage-2",
    ];
    stellar_build_utils::build(packages, ".stellar-contracts/proxy-deps");
}
