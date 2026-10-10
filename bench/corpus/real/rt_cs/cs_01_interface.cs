namespace Humanizer;

public interface IFormatter
{
    internal string DateHumanize_Now();

    internal string DateHumanize_Never();

    internal string DateHumanize(TimeUnit timeUnit, Tense timeUnitTense, int unit);

    internal string TimeSpanHumanize_Zero();

    internal string TimeSpanHumanize(TimeUnit timeUnit, int unit, bool toWords = false );

    internal string TimeSpanHumanize_Age();

    internal string DataUnitHumanize(DataUnit dataUnit, double count, bool toSymbol = true );

    internal string TimeUnitHumanize(TimeUnit timeUnit);

}
