use core::f64;

// The bucket boundary array `boundaries` has length `counts.len() + 1`
// - counts[0]: count of values in [0, boundaries[0])
// - counts[i]: count of values in [boundaries[i-1], boundaries[i]) (for i >= 1)
// TODO: Need to be more precise.
#[allow(dead_code)]
pub fn percentile(percent: f64, counts: &[u64], boundaries: &[f64]) -> Option<f64> {
    if boundaries.is_empty() || counts.len() != boundaries.len() + 1 {
        return None;
    }

    if !(0.0..=1.0).contains(&percent) {
        return None;
    }

    let total: u64 = counts.iter().sum();
    if total == 0 || percent == 0.0 {
        return None;
    }

    if percent == 1.0 {
        return Some(*boundaries.last().unwrap_or(&0.0));
    }

    let target_count = percent * (total as f64);

    counts
        .iter()
        .enumerate()
        .fold((0_u64, None), |(cumulative, result), (i, &count)| {
            let new_cumulative = cumulative + count;
            // Track cumulative count until the target is reached
            if result.is_none() && (new_cumulative as f64) >= target_count {
                let bucket_lower_count = cumulative;
                let bucket_lower_bound = if i == 0 { 0.0 } else { boundaries[i - 1] };
                let bucket_upper_bound = if i == boundaries.len() {
                    boundaries[i - 1]
                } else {
                    boundaries[i]
                };
                let fraction = if count == 0 {
                    0.0
                } else {
                    ((target_count - bucket_lower_count as f64) / count as f64).clamp(0.0, 1.0)
                };
                // This interpolated value will be returned
                let interpolated =
                    bucket_lower_bound + fraction * (bucket_upper_bound - bucket_lower_bound);
                (new_cumulative, Some(interpolated))
            } else {
                (new_cumulative, result)
            }
        })
        .1
        .or_else(|| Some(*boundaries.last().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_basic() {
        let counts = vec![1, 2, 3];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(0.5, &counts, &boundaries), Some(2.0));
    }

    #[test]
    fn test_percentile_empty_boundaries() {
        let counts = vec![1];
        let boundaries = vec![];
        assert_eq!(percentile(0.5, &counts, &boundaries), None);
    }

    #[test]
    fn test_percentile_invalid_counts_length() {
        let counts = vec![1, 2];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(0.5, &counts, &boundaries), None);
    }

    #[test]
    fn test_percentile_out_of_range() {
        let counts = vec![1, 2, 3];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(1.5, &counts, &boundaries), None);
    }

    #[test]
    fn test_percentile_zero_total_count() {
        let counts = vec![0, 0, 0];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(0.5, &counts, &boundaries), None);
    }

    #[test]
    fn test_percentile_exact_threshold() {
        let counts = vec![1, 2, 3];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(0.0, &counts, &boundaries), None);
        assert_eq!(percentile(1.0, &counts, &boundaries), Some(2.0));
    }

    #[test]
    fn test_percentile_linear_interpolation() {
        let counts = vec![1, 2, 3];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(0.25, &counts, &boundaries), Some(1.25));
        assert_eq!(percentile(0.75, &counts, &boundaries), Some(2.0));
        assert_eq!(percentile(0.90, &counts, &boundaries), Some(2.0));
    }

    #[test]
    fn test_percentile_large_counts() {
        let counts = vec![100, 200, 300];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(0.5, &counts, &boundaries), Some(2.0));
    }

    #[test]
    fn test_percentile_small_counts() {
        let counts = vec![1, 1, 1];
        let boundaries = vec![1.0, 2.0];
        assert_eq!(percentile(0.5, &counts, &boundaries), Some(1.5));
    }
}
