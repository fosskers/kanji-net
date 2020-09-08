//! Utility functions for handling Japanese text that isn't appropriate to
//! upstream into the `kanji` library.

pub fn is_voiced_pair(a: &str, b: &str) -> bool {
    let mut chars = a.chars().zip(b.chars());
    chars
        .next()
        .and_then(|(x, y)| voiced_char(x).map(|c| c == y))
        .unwrap_or(false)
        && chars.all(|(x, y)| x == y)
}

// は行 is excluded on purpose, since it doesn't follow proper voicing rules,
// and no 音読み start with P while on their own. Example: 一票 doesn't count
// since the P is "dynamic" from being paired with 一, and indeed dictionaries
// don't list ぴょう as a reading for 票.
fn voiced_char(c: char) -> Option<char> {
    match c {
        'か' => Some('が'),
        'き' => Some('ぎ'),
        'く' => Some('ぐ'),
        'け' => Some('げ'),
        'こ' => Some('ご'),
        'が' => Some('か'),
        'ぎ' => Some('き'),
        'ぐ' => Some('く'),
        'げ' => Some('け'),
        'ご' => Some('こ'),
        'さ' => Some('ざ'),
        'し' => Some('じ'),
        'す' => Some('ず'),
        'せ' => Some('ぜ'),
        'そ' => Some('ぞ'),
        'ざ' => Some('さ'),
        'じ' => Some('し'),
        'ず' => Some('す'),
        'ぜ' => Some('せ'),
        'ぞ' => Some('そ'),
        'た' => Some('だ'),
        'ち' => Some('ぢ'),
        'つ' => Some('づ'),
        'て' => Some('で'),
        'と' => Some('ど'),
        'だ' => Some('た'),
        'ぢ' => Some('ち'),
        'づ' => Some('つ'),
        'で' => Some('て'),
        'ど' => Some('と'),
        _ => None,
    }
}

pub fn is_rhyme(a: &str, b: &str) -> bool {
    let mut chars = a.chars().zip(b.chars());
    chars
        .next()
        .map(|(x, y)| vowel(x) == vowel(y))
        .unwrap_or(false)
        && chars.all(|(x, y)| x == y)
}

// TODO Account for small よ, etc.
/// What is the vowel of the given Hiragana?
fn vowel(c: char) -> Option<char> {
    match c {
        'あ' | 'か' | 'さ' | 'た' | 'な' | 'は' | 'ま' | 'や' | 'ら' | 'わ' => Some('あ'),
        'が' | 'ざ' | 'だ' | 'ば' | 'ぱ' => Some('あ'),
        'い' | 'き' | 'し' | 'ち' | 'に' | 'ひ' | 'み' | 'り' => Some('い'),
        'ぎ' | 'じ' | 'ぢ' | 'び' | 'ぴ' => Some('い'),
        'う' | 'く' | 'す' | 'つ' | 'ぬ' | 'ふ' | 'む' | 'ゆ' | 'る' => Some('う'),
        'ぐ' | 'ず' | 'づ' | 'ぶ' | 'ぷ' => Some('う'),
        'え' | 'け' | 'せ' | 'て' | 'ね' | 'へ' | 'め' | 'れ' => Some('え'),
        'げ' | 'ぜ' | 'で' | 'べ' | 'ぺ' => Some('え'),
        'お' | 'こ' | 'そ' | 'と' | 'の' | 'ほ' | 'も' | 'よ' | 'ろ' => Some('お'),
        'ご' | 'ぞ' | 'ど' | 'ぼ' | 'ぽ' => Some('お'),
        _ => None,
    }
}
