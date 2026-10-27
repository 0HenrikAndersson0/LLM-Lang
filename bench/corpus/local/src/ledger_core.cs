namespace Ledger.Core;

/// <summary>
/// The three-letter ISO currency code a ledger line is denominated in.
/// Codes are canon uppercase; validation happens at parse time.
/// </summary>
public enum Currency
{
    /// <summary>United States dollar.</summary>
    Usd,
    /// <summary>Euro.</summary>
    Eur,
    /// <summary>Japanese yen — zero-decimal in canon form.</summary>
    Jpy,
    /// <summary>Swedish krona — the ledger's home currency.</summary>
    Sek,
}

/// <summary>
/// An immutable monetary amount: minor units (cents, öre) plus the currency.
/// All arithmetic is checked; overflow throws rather than wraps, because a
/// silently wrapped ledger is worse than a crashed one.
/// </summary>
/// <param name="Minor">Minor units, always integral and signed.</param>
/// <param name="Curr">The ISO code carried on the amount.</param>
public record Money(long Minor, Currency Curr)
{
    /// <summary>Decimal scale per currency. Yen is zero-decimal by canon.</summary>
    public static readonly IReadOnlyDictionary<Currency, int> Scale =
        new Dictionary<Currency, int>
        {
            [Currency.Usd] = 2,
            [Currency.Eur] = 2,
            [Currency.Jpy] = 0,
            [Currency.Sek] = 2,
        };

    /// <summary>Builds an amount from a decimal in major units.</summary>
    /// <param name="amount">Major-unit value, e.g. 12.50.</param>
    /// <param name="curr">Currency to denominate in.</param>
    /// <exception cref="ArgumentOutOfRangeException">If <paramref name="amount"/>
    /// has more decimals than the currency's scale.</exception>
    public static Money Of(decimal amount, Currency curr)
    {
        var scale = Scale[curr];
        var minor = decimal.Round(amount, scale) * (decimal)Math.Pow(10, scale);
        if (minor % 1m != 0m)
        {
            throw new ArgumentOutOfRangeException(nameof(amount), amount, "residual decimals after scaling");
        }
        return new Money((long)minor, curr);
    }

    /// <summary>Returns the amount in major units as decimal.</summary>
    public decimal Major => Minor / (decimal)Math.Pow(10, Scale[Curr]);

    /// <summary>Checked addition; currencies must match.</summary>
    /// <exception cref="InvalidOperationException">currency mismatch</exception>
    public static Money operator +(Money a, Money b)
    {
        if (a.Curr != b.Curr)
        {
            throw new InvalidOperationException($"currency mismatch: {a.Curr} vs {b.Curr}");
        }
        return new Money(checked(a.Minor + b.Minor), a.Curr);
    }

    /// <summary>Adds only if currencies match — the safe API used by aggregation.</summary>
    public Result<Money, string> TryAdd(Money other) =>
        Curr == other.Curr
            ? Result<Money, string>.Ok(new Money(checked(Minor + other.Minor), Curr))
            : Result<Money, string>.Err($"currency mismatch: {Curr} vs {other.Curr}");
}

/// <summary>A minimal result type so aggregation never throws mid-batch.</summary>
public readonly struct Result<T, E>
{
    private readonly T? _ok;
    private readonly E? _err;

    private Result(T? ok, E? err, bool isOk)
    {
        IsOk = isOk;
        _ok = ok;
        _err = err;
    }

    /// <summary>True when the result carries a value.</summary>
    public bool IsOk { get; }

    /// <summary>Wraps a success value.</summary>
    public static Result<T, E> Ok(T value) => new(value, default, true);

    /// <summary>Wraps a failure value.</summary>
    public static Result<T, E> Err(E error) => new(default, error, false);

    /// <summary>Unwraps the success side; throws when the result is an error.</summary>
    public T Unwrap() =>
        IsOk ? _ok! : throw new InvalidOperationException($"unwrap on err: {_err}");

    /// <summary>Unwraps the failure side; throws when the result is a value.</summary>
    public E UnwrapErr() =>
        !IsOk ? _err! : throw new InvalidOperationException("unwrap_err on ok value");
}

/// <summary>Exchange rates are looked up through this seam, so tests can pin them.</summary>
public interface IRateProvider
{
    /// <summary>Latest minor-unit rate from one currency into another.</summary>
    /// <returns>The rate in minor units, or null when no route exists.</returns>
    long? Rate(Currency from, Currency to);
}