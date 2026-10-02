# Voxy wgpu 30.0.1 patch

CoreDevice equality/order/hash include the owning Global address and DeviceId.
DeviceId alone is local to an Instance, allowing independent contexts to compare
as equal and bypass engine device checks. The retained Arc keeps the address
stable for handle lifetime; clones have the same identity. This changes native
Device identity only. Other resource identity comparisons are not addressed.
Upstream licenses remain in this directory.
