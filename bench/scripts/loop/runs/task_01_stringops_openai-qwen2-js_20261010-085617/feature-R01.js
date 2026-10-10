export function CompactLabel(s) {
  // Trim surrounding whitespace
  s = s.trim();

  // If the result is empty, return '-'
  if (s === '') {
    return '-';
  }

  // Split on single spaces and keep only nonempty words
  const words = s.split(/\s+/).filter(word => word.length > 0);

  // Take the first character of each word, uppercased
  const initials = words.map(word => word[0].toUpperCase());

  // Concatenate those characters
  let result = initials.join('');

  // If the result is longer than 6 characters, return its first 6 characters
  if (result.length > 6) {
    result = result.slice(0, 6);
  }

  return result;
}
