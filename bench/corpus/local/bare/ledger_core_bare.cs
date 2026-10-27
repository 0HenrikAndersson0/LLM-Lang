namespace Ledger.Core;
public enum Currency
{
    Usd,
    Eur,
    Jpy,
    Sek,
}
public record Money(long Minor, Currency Curr)
{
    public static readonly IReadOnlyDictionary<Currency, int> Scale =
        new Dictionary<Currency, int>
        {
            [Currency.Usd] = 2,
            [Currency.Eur] = 2,
            [Currency.Jpy] = 0,
            [Currency.Sek] = 2,
        };
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
    public decimal Major => Minor / (decimal)Math.Pow(10, Scale[Curr]);
    public static Money operator +(Money a, Money b)
    {
        if (a.Curr != b.Curr)
        {
            throw new InvalidOperationException($"currency mismatch: {a.Curr} vs {b.Curr}");
        }
        return new Money(checked(a.Minor + b.Minor), a.Curr);
    }
    public Result<Money, string> TryAdd(Money other) =>
        Curr == other.Curr
            ? Result<Money, string>.Ok(new Money(checked(Minor + other.Minor), Curr))
            : Result<Money, string>.Err($"currency mismatch: {Curr} vs {other.Curr}");
}
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
    public bool IsOk { get; }
    public static Result<T, E> Ok(T value) => new(value, default, true);
    public static Result<T, E> Err(E error) => new(default, error, false);
    public T Unwrap() =>
        IsOk ? _ok! : throw new InvalidOperationException($"unwrap on err: {_err}");
    public E UnwrapErr() =>
        !IsOk ? _err! : throw new InvalidOperationException("unwrap_err on ok value");
}
public interface IRateProvider
{
    long? Rate(Currency from, Currency to);
}