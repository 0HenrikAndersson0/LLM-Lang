using Ledger.Core;

namespace Ledger.Service;

/// <summary>
/// Aggregation over ledger postings. The service is deliberately static:
/// the ledger is a pure fold over postings and every seam (caching, rates,
/// rounding) arrives as an injected record or interface.
/// </summary>
public static class LedgerService
{
    /// <summary>Default rounding mode when a caller does not care.</summary>
    public const RoundingMode DefaultRounding = RoundingMode.Bankers;

    /// <summary>
    /// Folds postings into a net position per currency. Postings that
    /// cannot be summed (currency mismatch inside a currency group) land
    /// in the error bucket and never abort the batch.
    /// </summary>
    /// <param name="postings">Postings to aggregate; empty input is legal.</param>
    /// <returns>A typed report carrying per-currency nets and failures.</returns>
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

    /// <summary>
    /// Converts a position into the home currency using the injected
    /// provider; an unknown route yields null rather than partial data.
    /// </summary>
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

    /// <summary>Rounds a major-unit flow under the requested mode.</summary>
    /// <exception cref="ArgumentOutOfRangeException">unknown mode</exception>
    public static decimal Round(decimal value, RoundingMode mode = DefaultRounding) =>
        mode switch
        {
            RoundingMode.Floor => decimal.Floor(value),
            RoundingMode.Ceiling => decimal.Ceiling(value),
            RoundingMode.Nearest => decimal.Round(value, MidpointRounding.AwayFromZero),
            RoundingMode.Bankers => decimal.Round(value, MidpointRounding.ToEven),
            _ => throw new ArgumentOutOfRangeException(nameof(mode), mode, "Unknown rounding mode."),
        };

    /// <summary>
    /// The posting audit trail: returns postings older than the cutoff,
    /// oldest first, skipping void ones — the shape the CSV exporter wants.
    /// </summary>
    public static IEnumerable<Posting> AuditTrail(
        IEnumerable<Posting> postings, DateTime cutoff) =>
        postings
            .Where(p => !p.Voided)
            .Where(p => p.BookedAt < cutoff)
            .OrderBy(p => p.BookedAt)
            .ThenBy(p => p.Amount.Minor);

    /// <summary>Human summary for the batch report header.</summary>
    public static string SummaryLine(ReconciliationReport<Currency> report) =>
        report.Failures.Count == 0
            ? $"{report.Nets.Count} currencies reconciled, 0 conflicts"
            : $"{report.Nets.Count} currencies reconciled, {report.Failures.Count} conflicts: {report.Failures.First()}";

    /// <summary>Parses a wire token like "SEK:1250" into an amount.</summary>
    /// <exception cref="FormatException">malformed token</exception>
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

/// <summary>A single ledger entry: the what, the when, and the how much.</summary>
/// <param name="Account">Post account, e.g. "acc:receivables".</param>
/// <param name="BookedAt">Booking instant (UTC); future bookings reject.</param>
/// <param name="Amount">Signed amount — negative is a debit.</param>
/// <param name="Voided">Void postings stay in the trail but skip aggregation.</param>
public record Posting(string Account, DateTime BookedAt, Money Amount, bool Voided = false)
{
    /// <summary>A posting that books in the future cannot exist; parse-time guard.</summary>
    public bool IsFuture => BookedAt > DateTime.UtcNow;
}

/// <summary>Outcome of folding a batch: nets per key plus what did not fold.</summary>
/// <param name="Nets">Per-currency net positions.</param>
/// <param name="Failures">Human-readable reasons for skipped postings.</param>
public record ReconciliationReport<K>(IReadOnlyDictionary<K, Money> Nets, IReadOnlyList<string> Failures)
{
    /// <summary>True when the batch folded cleanly.</summary>
    public bool Clean => Failures.Count == 0;
}

/// <summary>How <see cref="LedgerService.Round"/> treats the midpoint.</summary>
public enum RoundingMode
{
    /// <summary>Toward negative infinity.</summary>
    Floor,
    /// <summary>Toward positive infinity.</summary>
    Ceiling,
    /// <summary>0.5 goes away from zero — merchant expectation.</summary>
    Nearest,
    /// <summary>0.5 goes to even — banker's rounding.</summary>
    Bankers,
}