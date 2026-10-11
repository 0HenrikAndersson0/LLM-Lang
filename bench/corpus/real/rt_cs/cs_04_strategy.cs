namespace Humanizer;

public class DefaultDateTimeHumanizeStrategy : IDateTimeHumanizeStrategy
{
    public string Humanize(DateTime input, DateTime comparisonBase, CultureInfo? culture) =>
        DateTimeHumanizeAlgorithms.DefaultHumanize(input, comparisonBase, culture);
}
