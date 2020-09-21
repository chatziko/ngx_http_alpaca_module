//! Provides functions to sample objects' count and size
//!	using the ALPaCA's deterministic way.
use rand_distr::Distribution;

/// Returns the next multiple of "num" which is greater
/// or equal than "min".
pub fn get_multiple(num: usize, min: usize) -> usize {
    let mut count = num;

    while count < min {
        count += num;
    }

    count
}

/// Returns a vector of target sizes for the fake objects. Sizes have
/// to be a multiple of "obj_size" and smaller than "max_obj_size".
/// They are sampled uniformly.
pub fn get_multiples_in_range(
    obj_size: usize,
    max_obj_size: usize,
    n: usize,
) -> Result<Vec<usize>, String> {
    if (obj_size > max_obj_size) || (max_obj_size % obj_size != 0) {
        return Err(format!("max_obj_size ({}) must be greater-or-equal and a multiple of obj_size ({})", max_obj_size, obj_size));
    }

    let mut sizes: Vec<usize> = Vec::with_capacity(n); // Vector of target sizes.

    let max = max_obj_size / obj_size + 1;
    let between = rand_distr::Uniform::from(1..max);

    for _ in 0..n {
        let num: usize = between.sample(&mut rand::thread_rng());
        sizes.push(num * obj_size);
    }

    Ok(sizes)
}
