# Rust SIMD Experiments

A huge problem in modern CPUs is the massive difference in latencies of in
computation in comparison to memory access. Memory accesses are a couple of
orders of magnitude slower than basic CPU instructions.

This problem dominates the runtime of naively implemented algorithms. Where
cache misses cause the runtime to repeatedly access main memory. Algorithms
must instead be designed to reduce the number of memory accesses as much as
possible.

For problems that have significant data dependencies, such as the Fast Multipole
Method (FMM), this becomes challenging. A naive implementation will potentially result
in a cache miss for every iteration, as data will have to be looked up from anywhere
in global buffer.

In this crate I try out a few different experiments to alleviate global memory
access problems for a problem that has a similar logic to the field translations
in the FMM.

A strategy for coping with this is to gather mutable pointers to save locations
and explicitly load them into SIMD registers, while computing the results of the
field translation in place in a vectorised manner.


## Important SIMD terminology

This isn't a resource on SIMD programming, but some notes that I've made of things
that I need to commit to memory.

* Vector - a SIMD value is called a vector. It's of fixed size known at compile
time. Vectors are aligned with respect to their entire size (similar, but different
concept to rust stack allocated arrays)

* Lane - A single element position within a SIMD vector is called a lane. Accessing
individual lane values is relatively expensive. A SIMD vector has to be pushed out of
the register and onto the stack before an individual lane can be accessed on most
architectures. The vector then has to potentially be pushed back onto the stack,
lane access should be in general avoided in any hot loop.

* Bit Widths - the bit widths in this context are the size of the vectors involved,
not the individual elements.

* Vector Register - name for the extra wide registers used for SIMD operations.
Sometimes called floating point registers as it's common for the registers to be
used with both scalar and vectorised floating point operations.

* Vertical - when an operation is vertical, each lane processes individually without
regard to the other lanes in the same vector. Most SIMD operations are vertical.

* Reducing/Reduce - `reduce_*` operations, the lanes within a single vector are
merged using some operation.

* Target Feature - Rust calls a CPU architecture extension a `target_feature`.

Nothing protects you from running binaries not built for your architecture, this
is automatically undefined behaviour.

