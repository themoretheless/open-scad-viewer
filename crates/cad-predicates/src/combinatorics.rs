//! Binomial coefficients shared by exact predicates.
const BINOMIAL_TABLE: [[u64; 33]; 33] = {
    let mut table = [[0u64; 33]; 33];
    let mut n = 0;
    while n < 33 {
        table[n][0] = 1;
        let mut k = 1;
        while k <= n {
            table[n][k] = table[n - 1][k - 1] + table[n - 1][k];
            k += 1;
        }
        n += 1;
    }
    table
};

#[inline(always)]
pub(crate) fn binomial_u64(n: usize, k: usize) -> u64 {
    if k > n {
        0
    } else if n < 33 {
        BINOMIAL_TABLE[n][k]
    } else {
        (0..k).fold(1, |v, i| v * (n - i) as u64 / (i + 1) as u64)
    }
}
