fn main() {
    // The DiamondManager requires the DiamondProxy facet built
    let extra_packages = vec![
        "stellar-diamond-proxy",
        "shared-storage-1",
        "shared-storage-2",
    ];
    stellar_build_utils::build(extra_packages, ".stellar-contracts/manager-deps");
}
