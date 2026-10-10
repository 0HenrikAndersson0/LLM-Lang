namespace Acme.Labels;

internal static partial class ModuleTop
{
    public static string CompactLabel(string s)
    {
        var trimmed = s.Trim();
        if (trimmed.Length==0)
        {
            return "-";
        }
        var chars = trimmed.Split(" ");
        var result = string.Join("", chars);
        if (result.Length>6)
        {
            return result.Substring(0, 6);
        }
        return result;
    }
}
