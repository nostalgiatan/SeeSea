//! Look-alike names: one or two typos away from a very popular site, or a famous brand's name used
//! inside a domain that is not the brand's.

/// Optimal string alignment distance (insert, delete, substitute, swap neighbours), stopping early
/// once it is certain to exceed `max`.
pub fn distance(a: &str, b: &str, max: usize) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > max {
        return max + 1;
    }
    let (n, m) = (a.len(), b.len());
    // Whatever the edits are, the characters before the first and after the last one still agree;
    // a swap of neighbours breaks two positions, so each allowed edit may cost two.
    let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    if prefix + suffix + 2 * max < n.min(m) {
        return max + 1;
    }
    let mut d = vec![vec![0usize; m + 1]; n + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for j in 0..=m {
        d[0][j] = j;
    }
    for i in 1..=n {
        let mut best = usize::MAX;
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut v = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                v = v.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = v;
            best = best.min(v);
        }
        if best > max {
            return max + 1;
        }
    }
    d[n][m]
}

/// Letters that look like others, folded so `paypa1` and `rnicrosoft` compare as the real names.
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '0' => out.push('o'),
            '1' | 'l' | '|' => out.push('l'),
            '5' => out.push('s'),
            '$' => out.push('s'),
            '3' => out.push('e'),
            'r' if it.peek() == Some(&'n') => {
                it.next();
                out.push('m');
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_counts_typos_and_swaps() {
        assert_eq!(distance("google", "google", 2), 0);
        assert_eq!(distance("gogle", "google", 2), 1);
        assert_eq!(distance("googel", "google", 2), 1);
        assert_eq!(distance("gooogle", "google", 2), 1);
        assert_eq!(distance("goggle", "google", 2), 1);
        assert_eq!(distance("amazon", "google", 2), 3);
        assert_eq!(distance("a", "abcdef", 2), 3);
    }

    #[test]
    fn look_alike_letters_fold_to_the_real_ones() {
        assert_eq!(fold("paypa1"), "paypal");
        assert_eq!(fold("rnicrosoft"), "microsoft");
        assert_eq!(fold("g00gle"), "google");
    }
}
