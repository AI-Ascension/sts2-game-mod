// SPDX-License-Identifier: MIT

pub(super) fn bearer_token_matches(candidate: Option<&String>, token: &[u8]) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    let candidate = candidate.as_bytes();
    let prefix = b"Bearer ";
    let expected_length = prefix.len() + token.len();
    let comparison_length = candidate.len().max(expected_length);
    let mut difference = candidate.len() ^ expected_length;

    for index in 0..comparison_length {
        let actual = candidate.get(index).copied().unwrap_or(0);
        let expected = if index < prefix.len() {
            prefix[index]
        } else {
            token.get(index - prefix.len()).copied().unwrap_or(0)
        };
        difference |= usize::from(actual ^ expected);
    }

    difference == 0
}

#[cfg(test)]
mod tests {
    use super::bearer_token_matches;

    #[test]
    fn bearer_token_requires_exact_prefix_and_secret() {
        let token = b"secret-token";
        let valid = String::from("Bearer secret-token");
        assert!(bearer_token_matches(Some(&valid), token));

        for invalid in [
            String::from("Bearer secret-toke"),
            String::from("Bearer secret-token-extra"),
            String::from("bearer secret-token"),
            String::from("Basic secret-token"),
        ] {
            assert!(!bearer_token_matches(Some(&invalid), token));
        }
        assert!(!bearer_token_matches(None, token));
    }
}
