using Ledger.Core;

namespace Ledger.Service;
public static class LedgerService
{
    public const RoundingMode DefaultRounding = RoundingMode.Bankers;
    public static ReconciliationReport<Currency> Net(IEnumerable<Posting> postings)
    {
        var nets = new Dictionary<Currency, Money>();
        var failures = new List<string>();
        foreach (var group in postings.GroupBy(p => p.Amount.Curr))
        {
            var net = new Money(0, group.Key);
            foreach (var p in group)
            {
                var sum = net.TryAdd(p.Amount);
                if (sum.IsOk)
                {
                    net = sum.Unwrap();
                }
                else
                {
                    failures.Add($"{group.Key}: {sum.UnwrapErr()}");
                }
            }
            nets[group.Key] = net;
        }
        return new ReconciliationReport<Currency>(nets, failures);
    }
    public static Money? ConvertTo(Money amount, Currency home, IRateProvider rates)
    {
        if (amount.Curr == home)
        {
            return amount;
        }
        var rate = rates.Rate(amount.Curr, home);
        return rate is null
            ? null
            : new Money(checked((long)Math.Round(amount.Minor * (double)rate)), home);
    }
    public static decimal Round(decimal value, RoundingMode mode = DefaultRounding) =>
        mode switch
        {
            RoundingMode.Floor => decimal.Floor(value),
            RoundingMode.Ceiling => decimal.Ceiling(value),
            RoundingMode.Nearest => decimal.Round(value, MidpointRounding.AwayFromZero),
            RoundingMode.Bankers => decimal.Round(value, MidpointRounding.ToEven),
            _ => throw new ArgumentOutOfRangeException(nameof(mode), mode, "Unknown rounding mode."),
        };
    public static IEnumerable<Posting> AuditTrail(
        IEnumerable<Posting> postings, DateTime cutoff) =>
        postings
            .Where(p => !p.Voided)
            .Where(p => p.BookedAt < cutoff)
            .OrderBy(p => p.BookedAt)
            .ThenBy(p => p.Amount.Minor);
    public static string SummaryLine(ReconciliationReport<Currency> report) =>
        report.Failures.Count == 0
            ? $"{report.Nets.Count} currencies reconciled, 0 conflicts"
            : $"{report.Nets.Count} currencies reconciled, {report.Failures.Count} conflicts: {report.Failures.First()}";
    public static Money Parse(string token)
    {
        var parts = token.Split(':');
        if (parts.Length != 2)
        {
            throw new FormatException($"expected CURR:minor, got '{token}'");
        }
        if (!Enum.TryParse<Currency>(parts[0], ignoreCase: false, out var curr))
        {
            throw new FormatException($"unknown currency '{parts[0]}'");
        }
        return new Money(long.Parse(parts[1]), curr);
    }
}
public record Posting(string Account, DateTime BookedAt, Money Amount, bool Voided = false)
{
    public bool IsFuture => BookedAt > DateTime.UtcNow;
}
public record ReconciliationReport<K>(IReadOnlyDictionary<K, Money> Nets, IReadOnlyList<string> Failures)
{
    public bool Clean => Failures.Count == 0;
}
public enum RoundingMode
{
    Floor,
    Ceiling,
    Nearest,
    Bankers,
}