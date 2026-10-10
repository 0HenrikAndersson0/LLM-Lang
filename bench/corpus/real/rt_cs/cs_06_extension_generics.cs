namespace Humanizer;

public static class ByteSizeExtensions
{
    static ByteSize Bits(this Byte input) =>
        ByteSize.FromBits(input);
    static ByteSize Bits(this SByte input) =>
        ByteSize.FromBits(input);
    static ByteSize Bits(this Short input) =>
        ByteSize.FromBits(input);
    static ByteSize Bits(this UShort input) =>
        ByteSize.FromBits(input);
    static ByteSize Bits(this int input) =>
        ByteSize.FromBits(input);
    static ByteSize Bits(this UInt input) =>
        ByteSize.FromBits(input);
    static ByteSize Bits(this long input) =>
        ByteSize.FromBits(input);
    static ByteSize Bits(this double input) =>
        ByteSize.FromBytes(input/ByteSize.BitsInByte);
    static ByteSize Bytes(this Byte input) =>
        ByteSize.FromBytes(input);
    static ByteSize Bytes(this SByte input) =>
        ByteSize.FromBytes(input);
    static ByteSize Bytes(this Short input) =>
        ByteSize.FromBytes(input);
    static ByteSize Bytes(this UShort input) =>
        ByteSize.FromBytes(input);
    static ByteSize Bytes(this int input) =>
        ByteSize.FromBytes(input);
    static ByteSize Bytes(this UInt input) =>
        ByteSize.FromBytes(input);
    static ByteSize Bytes(this double input) =>
        ByteSize.FromBytes(input);
    static ByteSize Bytes(this long input) =>
        ByteSize.FromBytes(input);
    static ByteSize Kilobytes(this Byte input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Kilobytes(this SByte input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Kilobytes(this Short input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Kilobytes(this UShort input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Kilobytes(this int input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Kilobytes(this UInt input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Kilobytes(this double input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Kilobytes(this long input) =>
        ByteSize.FromKilobytes(input);
    static ByteSize Megabytes(this Byte input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Megabytes(this SByte input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Megabytes(this Short input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Megabytes(this UShort input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Megabytes(this int input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Megabytes(this UInt input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Megabytes(this double input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Megabytes(this long input) =>
        ByteSize.FromMegabytes(input);
    static ByteSize Gigabytes(this Byte input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Gigabytes(this SByte input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Gigabytes(this Short input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Gigabytes(this UShort input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Gigabytes(this int input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Gigabytes(this UInt input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Gigabytes(this double input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Gigabytes(this long input) =>
        ByteSize.FromGigabytes(input);
    static ByteSize Terabytes(this Byte input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Terabytes(this SByte input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Terabytes(this Short input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Terabytes(this UShort input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Terabytes(this int input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Terabytes(this UInt input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Terabytes(this double input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Terabytes(this long input) =>
        ByteSize.FromTerabytes(input);
    static ByteSize Petabytes(this Byte input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Petabytes(this SByte input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Petabytes(this Short input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Petabytes(this UShort input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Petabytes(this int input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Petabytes(this UInt input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Petabytes(this double input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Petabytes(this long input) =>
        ByteSize.FromPetabytes(input);
    static ByteSize Exabytes(this Byte input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Exabytes(this SByte input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Exabytes(this Short input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Exabytes(this UShort input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Exabytes(this int input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Exabytes(this UInt input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Exabytes(this double input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Exabytes(this long input) =>
        ByteSize.FromExabytes(input);
    static ByteSize Pebibytes(this Byte input) =>
        ByteSize.FromPebibytes(input);
    static ByteSize Pebibytes(this SByte input) =>
        ByteSize.FromPebibytes(input);
    static ByteSize Pebibytes(this Short input) =>
        ByteSize.FromPebibytes(input);
    static ByteSize Pebibytes(this UShort input) =>
        ByteSize.FromPebibytes(input);
    static ByteSize Pebibytes(this int input) =>
        ByteSize.FromPebibytes(input);
    static ByteSize Pebibytes(this UInt input) =>
        ByteSize.FromPebibytes(input);
    static ByteSize Pebibytes(this double input) =>
        ByteSize.FromPebibytes(input);
    static ByteSize Pebibytes(this long input) =>
        ByteSize.FromPebibytes(input);
    static string Humanize(this ByteSize input, string? format = null ) =>
        string.IsNullOrWhiteSpace(format)?input.ToString(): input.ToString(format);
    static string Humanize(this ByteSize input, IFormatProvider formatProvider) =>
        input.ToString(formatProvider);
    static string Humanize(this ByteSize input, string? format, IFormatProvider? formatProvider) =>
        string.IsNullOrWhiteSpace(format)?input.ToString(formatProvider): input.ToString(format, formatProvider);
    static string HumanizeWithUnitSystem(this ByteSize input, ByteSizeUnitSystem unitSystem, string? format = null , IFormatProvider? formatProvider = null ) =>
        unitSystem==ByteSizeUnitSystem.Legacy?input.Humanize(format, formatProvider): input.Format(unitSystem, format, formatProvider);
    static string HumanizeComposite(this ByteSize input, int precision = 2, IFormatProvider? formatProvider = null , string separator = " ", bool toWords = false )
    {
        if (precision<1)
        {
            throw new ArgumentOutOfRangeException(nameof (precision), precision, "");
        }
        ArgumentNullException.ThrowIfNull(separator);
        formatProvider??=CultureInfo.CurrentCulture;
        var culture = formatProvider as CultureInfo;
        if (culture is not null )
        {
            formatProvider=LocaleNumberFormattingOverrides.GetFormattingNumberFormat(culture);
        }
        var formatter = Configurator.GetFormatter(culture);
        var isNegative = input.Bits<0;
        var remainingBits = isNegative?(Int64)+1: (Int64);
        if (remainingBits==0)
        {
            return string.Concat("", formatter.DataUnitHumanize(DataUnit.Bit, 0, toSymbol));
        }
        var parts = new List<String>(Math.Min(precision, 8));
        void addPart(long bitsPerUnit, DataUnit unit)
    {
        if (parts.Count>=precision)
        {
            return ;
        }
        var count = remainingBits/bitsPerUnit;
        if (count==0)
        {
            return ;
        }
        parts.Add(string.Concat(count.ToString(formatProvider), "", formatter.DataUnitHumanize(unit, count, toSymbol)));
        remainingBits%=bitsPerUnit;
    }
        addPart((Int64)*ByteSize.BitsInByte, DataUnit.Exabyte);
        addPart((Int64)*ByteSize.BitsInByte, DataUnit.Petabyte);
        addPart((Int64)*ByteSize.BitsInByte, DataUnit.Terabyte);
        addPart((Int64)*ByteSize.BitsInByte, DataUnit.Gigabyte);
        addPart((Int64)*ByteSize.BitsInByte, DataUnit.Megabyte);
        addPart((Int64)*ByteSize.BitsInByte, DataUnit.Kilobyte);
        addPart(ByteSize.BitsInByte, DataUnit.Byte);
        addPart(1, DataUnit.Bit);
        return string.Concat(isNegative?NumberFormatInfo.GetInstance(formatProvider).NegativeSign: null, string.Join(separator, parts));
    }
    static string HumanizeCompositeWithUnitSystem(this ByteSize input, ByteSizeUnitSystem unitSystem, int precision = 2, IFormatProvider? formatProvider = null , string separator = " ", bool toWords = false )
    {
        if (unitSystem==ByteSizeUnitSystem.Legacy)
        {
            return input.HumanizeComposite(precision, formatProvider, separator, toWords);
        }
        if (unitSystem is <ByteSizeUnitSystem.DecimalSi or >ByteSizeUnitSystem.BinaryIec)
        {
            throw new ArgumentOutOfRangeException(nameof (unitSystem), unitSystem, "");
        }
        if (precision<1)
        {
            throw new ArgumentOutOfRangeException(nameof (precision), precision, "");
        }
        ArgumentNullException.ThrowIfNull(separator);
        formatProvider??=CultureInfo.CurrentCulture;
        var culture = formatProvider as CultureInfo;
        if (culture is not null )
        {
            formatProvider=LocaleNumberFormattingOverrides.GetFormattingNumberFormat(culture);
        }
        var formatter = Configurator.GetFormatter(culture);
        var builtInFormatter = formatter as DefaultFormatter;
        if (builtInFormatter is not null &&formatter.GetType.Assembly!=typeofDefaultFormatter.Assembly)
        {
            builtInFormatter=null ;
        }
        var isNegative = input.Bits<0;
        var remainingBits = isNegative?(Int64)+1: (Int64);
        var units = unitSystem==ByteSizeUnitSystem.DecimalSi?DecimalCompositeUnits: BinaryCompositeUnits;
        if (remainingBits==0)
        {
            return string.Concat("", toWords?humanizeUnit(DataUnit.Bit, 0): ByteSize.BitSymbol);
        }
        var parts = new List<String>(Math.Min(precision, units.Length+2));
        foreach (var unit in units)
        {
            if (parts.Count>=precision)
            {
                break;
            }
            var bitsPerUnit = (Int64)*ByteSize.BitsInByte;
            var count = remainingBits/bitsPerUnit;
            if (count==0)
            {
                continue;
            }
            var unitText = toWords?humanizeUnit(unit.DataUnit, count): unit.Symbol;
            parts.Add(string.Concat(count.ToString(formatProvider), "", unitText));
            remainingBits%=bitsPerUnit;
        }
        addSmallUnit(ByteSize.BitsInByte, ByteSize.ByteSymbol, DataUnit.Byte);
        addSmallUnit(1, ByteSize.BitSymbol, DataUnit.Bit);
        return string.Concat(isNegative?NumberFormatInfo.GetInstance(formatProvider).NegativeSign: null, string.Join(separator, parts));
        void addSmallUnit(long bitsPerUnit, string symbol, DataUnit dataUnit)
    {
        if (parts.Count>=precision)
        {
            return ;
        }
        var count = remainingBits/bitsPerUnit;
        if (count==0)
        {
            return ;
        }
        parts.Add(string.Concat(count.ToString(formatProvider), "", toWords?humanizeUnit(dataUnit, count): symbol));
        remainingBits%=bitsPerUnit;
    }
        void humanizeUnit(DataUnit dataUnit, long count)
    {
        builtInFormatter is not null ?builtInFormatter.DataUnitHumanizeExact(dataUnit, count, toSymbol): formatter.DataUnitHumanize(dataUnit, count, toSymbol);
    }
    }
    class CompositeUnit
    {
    }
    static ByteRate Per(this ByteSize size, TimeSpan interval) =>
        new ();
}
