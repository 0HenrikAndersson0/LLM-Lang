```csharp
using System;

public static class CompactLabelOps
{
    public static string CompactLabel(string s)
    {
        s = s.Trim();
        if (string.IsNullOrEmpty(s))
        {
            return "-";
        }

        string[] words = s.Split(' ', StringSplitOptions.RemoveEmptyEntries);
        string result = string.Concat(words.Select(word => char.ToUpperInvariant(word[0])));

        return result.Length > 6 ? result.Substring(0, 6) : result;
    }
}
```