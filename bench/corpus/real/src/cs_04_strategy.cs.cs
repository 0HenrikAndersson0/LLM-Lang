namespace Humanizer;

public class DefaultDateTimeHumanizeStrategy
{
    string Humanize(DateTime input, DateTime comparisonBase, CultureInfo? culture) =>
        DateTimeHumanizeAlgorithms.DefaultHumanize(input, comparisonBase, culture);
}
