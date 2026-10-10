export function ParseDuration(s) {
    if (s.length === 0) return 0;

    let totalSeconds = 0;
    let i = 0;

    while (i < s.length) {
        // Find the end of the digit run
        let digitEnd = i;
        while (digitEnd < s.length && s[digitEnd] >= '0' && s[digitEnd] <= '9') {
            digitEnd++;
        }

        // Extract the digits and convert to an integer
        if (digitEnd > i) {
            let digits = s.slice(i, digitEnd);
            let value = parseInt(digits, 10);

            // Find the unit character
            if (digitEnd < s.length) {
                let unit = s[digitEnd];
                if (unit === 'h') {
                    totalSeconds += value * 3600;
                } else if (unit === 'm') {
                    totalSeconds += value * 60;
                } else if (unit === 's') {
                    totalSeconds += value;
                } else {
                    return 0; // Invalid unit
                }
            } else {
                return 0; // Missing unit
            }
        } else {
            return 0; // No digits in segment
        }

        // Move to the next segment
        i = digitEnd + 1;
    }

    return totalSeconds;
}
