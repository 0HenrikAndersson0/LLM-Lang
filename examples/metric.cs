namespace Examples.Metric;

internal static partial class ModuleTop
{
    public static double BuildNumber(string input, char last)
    {
        return char.IsLetter(last)?BuildMetricNumber(input, last): double.Parse(input);
    }
    public static double BuildMetricNumber(string input, char last)
    {
        var number = double.Parse(input[..^1]);
        var indexInPositive = Symbols[0].IndexOf(last);
        if (indexInPositive>=0)
        {
            var exponent = (indexInPositive+1)*3.0;
            return number*Math.Pow(10, exponent);
        }
        var indexInNegative = Symbols[1].IndexOf(last);
        var exponent2 = (indexInNegative+1)*3.0;
        return number*Math.Pow(10, -exponent2);
    }
}
