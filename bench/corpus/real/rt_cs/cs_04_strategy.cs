namespace Humanizer;

public class DefaultDateTimeHumanizeStrategy
{
    public string Humanize(DateTime input, DateTime comparisonBase, CultureInfo? culture) =>
        DateTimeHumanizeAlgorithms.DefaultHumanize(input, comparisonBase, culture);
}
