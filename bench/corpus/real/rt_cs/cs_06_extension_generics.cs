namespace Humanizer;

public static class ByteSizeExtensions
{
    public static ByteSize Bits(this byte input) =>
        ByteSize.FromBits(input);
    public static ByteSize Bits(this sbyte input) =>
        ByteSize.FromBits(input);
    public static ByteSize Bits(this short input) =>
        ByteSize.FromBits(input);
    public static ByteSize Bits(this ushort input) =>
        ByteSize.FromBits(input);
    public static ByteSize Bits(this int input) =>
        ByteSize.FromBits(input);
    public static ByteSize Bits(this uint input) =>
        ByteSize.FromBits(input);
    public static ByteSize Bits(this long input) =>
        ByteSize.FromBits(input);
    public static ByteSize Bits(this double input) =>
        ByteSize.FromBytes(input/ByteSize.BitsInByte);
    public static ByteSize Bytes(this byte input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Bytes(this sbyte input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Bytes(this short input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Bytes(this ushort input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Bytes(this int input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Bytes(this uint input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Bytes(this double input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Bytes(this long input) =>
        ByteSize.FromBytes(input);
    public static ByteSize Kilobytes(this byte input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Kilobytes(this sbyte input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Kilobytes(this short input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Kilobytes(this ushort input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Kilobytes(this int input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Kilobytes(this uint input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Kilobytes(this double input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Kilobytes(this long input) =>
        ByteSize.FromKilobytes(input);
    public static ByteSize Megabytes(this byte input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Megabytes(this sbyte input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Megabytes(this short input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Megabytes(this ushort input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Megabytes(this int input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Megabytes(this uint input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Megabytes(this double input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Megabytes(this long input) =>
        ByteSize.FromMegabytes(input);
    public static ByteSize Gigabytes(this byte input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Gigabytes(this sbyte input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Gigabytes(this short input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Gigabytes(this ushort input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Gigabytes(this int input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Gigabytes(this uint input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Gigabytes(this double input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Gigabytes(this long input) =>
        ByteSize.FromGigabytes(input);
    public static ByteSize Terabytes(this byte input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Terabytes(this sbyte input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Terabytes(this short input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Terabytes(this ushort input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Terabytes(this int input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Terabytes(this uint input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Terabytes(this double input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Terabytes(this long input) =>
        ByteSize.FromTerabytes(input);
    public static ByteSize Petabytes(this byte input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Petabytes(this sbyte input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Petabytes(this short input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Petabytes(this ushort input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Petabytes(this int input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Petabytes(this uint input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Petabytes(this double input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Petabytes(this long input) =>
        ByteSize.FromPetabytes(input);
    public static ByteSize Exabytes(this byte input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Exabytes(this sbyte input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Exabytes(this short input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Exabytes(this ushort input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Exabytes(this int input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Exabytes(this uint input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Exabytes(this double input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Exabytes(this long input) =>
        ByteSize.FromExabytes(input);
    public static ByteSize Pebibytes(this byte input) =>
        ByteSize.FromPebibytes(input);
    public static ByteSize Pebibytes(this sbyte input) =>
        ByteSize.FromPebibytes(input);
    public static ByteSize Pebibytes(this short input) =>
        ByteSize.FromPebibytes(input);
    public static ByteSize Pebibytes(this ushort input) =>
        ByteSize.FromPebibytes(input);
    public static ByteSize Pebibytes(this int input) =>
        ByteSize.FromPebibytes(input);
    public static ByteSize Pebibytes(this uint input) =>
        ByteSize.FromPebibytes(input);
    public static ByteSize Pebibytes(this double input) =>
        ByteSize.FromPebibytes(input);
    public static ByteSize Pebibytes(this long input) =>
        ByteSize.FromPebibytes(input);
    public static string Humanize(this ByteSize input, string? format = null ) =>
        string.IsNullOrWhiteSpace(format)?input.ToString(): input.ToString(format);
    public static string Humanize(this ByteSize input, IFormatProvider formatProvider) =>
        input.ToString(formatProvider);
    public static string Humanize(this ByteSize input, string? format, IFormatProvider? formatProvider) =>
        string.IsNullOrWhiteSpace(format)?input.ToString(formatProvider): input.ToString(format, formatProvider);
    public static string HumanizeWithUnitSystem(this ByteSize input, ByteSizeUnitSystem unitSystem, string? format = null , IFormatProvider? formatProvider = null ) =>
        unitSystem==ByteSizeUnitSystem.Legacy?input.Humanize(format, formatProvider): input.Format(unitSystem, format, formatProvider);
    public static string HumanizeComposite(this ByteSize input, int precision = 2, IFormatProvider? formatProvider = null , string separator = " ", bool toWords = false )
    {
        if (precision<1)
        {
            throw new ArgumentOutOfRangeException(nameof (precision), precision, "Precision must be greater than zero.");
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
        var remainingBits = isNegative?(Int64)(-(input.Bits+1))+1: (long)input.Bits;
        if (remainingBits==0)
        {
            return string.Concat("0 ", formatter.DataUnitHumanize(DataUnit.Bit, 0, toSymbol: !toWords));
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
        parts.Add(string.Concat(count.ToString(formatProvider), " ", formatter.DataUnitHumanize(unit, count, toSymbol: !toWords)));
        remainingBits%=bitsPerUnit;
    }
        addPart((long)ByteSize.BytesInExabyte*ByteSize.BitsInByte, DataUnit.Exabyte);
        addPart((long)ByteSize.BytesInPetabyte*ByteSize.BitsInByte, DataUnit.Petabyte);
        addPart((long)ByteSize.BytesInTerabyte*ByteSize.BitsInByte, DataUnit.Terabyte);
        addPart((long)ByteSize.BytesInGigabyte*ByteSize.BitsInByte, DataUnit.Gigabyte);
        addPart((long)ByteSize.BytesInMegabyte*ByteSize.BitsInByte, DataUnit.Megabyte);
        addPart((long)ByteSize.BytesInKilobyte*ByteSize.BitsInByte, DataUnit.Kilobyte);
        addPart(ByteSize.BitsInByte, DataUnit.Byte);
        addPart(1, DataUnit.Bit);
        return string.Concat(isNegative?NumberFormatInfo.GetInstance(formatProvider).NegativeSign: null, string.Join(separator, parts));
    }
    public static string HumanizeCompositeWithUnitSystem(this ByteSize input, ByteSizeUnitSystem unitSystem, int precision = 2, IFormatProvider? formatProvider = null , string separator = " ", bool toWords = false )
    {
        if (unitSystem==ByteSizeUnitSystem.Legacy)
        {
            return input.HumanizeComposite(precision, formatProvider, separator, toWords);
        }
        if (unitSystem is <ByteSizeUnitSystem.DecimalSi or >ByteSizeUnitSystem.BinaryIec)
        {
            throw new ArgumentOutOfRangeException(nameof (unitSystem), unitSystem, "Unknown byte-size unit system.");
        }
        if (precision<1)
        {
            throw new ArgumentOutOfRangeException(nameof (precision), precision, "Precision must be greater than zero.");
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
        if (builtInFormatter is not null &&formatter.GetType().Assembly!=typeof (DefaultFormatter).Assembly)
        {
            builtInFormatter=null ;
        }
        var isNegative = input.Bits<0;
        var remainingBits = isNegative?(Int64)(-(input.Bits+1))+1: (long)input.Bits;
        var units = unitSystem==ByteSizeUnitSystem.DecimalSi?DecimalCompositeUnits: BinaryCompositeUnits;
        if (remainingBits==0)
        {
            return string.Concat("0 ", toWords?humanizeUnit(DataUnit.Bit, 0): ByteSize.BitSymbol);
        }
        var parts = new List<String>(Math.Min(precision, units.Length+2));
        foreach (var unit in units)
        {
            if (parts.Count>=precision)
            {
                break;
            }
            var bitsPerUnit = (long)unit.Bytes*ByteSize.BitsInByte;
            var count = remainingBits/bitsPerUnit;
            if (count==0)
            {
                continue;
            }
            var unitText = toWords?humanizeUnit(unit.DataUnit, count): unit.Symbol;
            parts.Add(string.Concat(count.ToString(formatProvider), " ", unitText));
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
        parts.Add(string.Concat(count.ToString(formatProvider), " ", toWords?humanizeUnit(dataUnit, count): symbol));
        remainingBits%=bitsPerUnit;
    }
        string humanizeUnit(DataUnit dataUnit, long count) =>
        builtInFormatter is not null ?builtInFormatter.DataUnitHumanizeExact(dataUnit, count, toSymbol: false ): formatter.DataUnitHumanize(dataUnit, count, toSymbol: false );
    }
    static readonly CompositeUnit[] DecimalCompositeUnits = [new (ByteSize.BytesInDecimalExabyte, "EB", DataUnit.DecimalExabyte), new (ByteSize.BytesInDecimalPetabyte, "PB", DataUnit.DecimalPetabyte), new (ByteSize.BytesInDecimalTerabyte, "TB", DataUnit.DecimalTerabyte), new (ByteSize.BytesInDecimalGigabyte, "GB", DataUnit.DecimalGigabyte), new (ByteSize.BytesInDecimalMegabyte, "MB", DataUnit.DecimalMegabyte), new (ByteSize.BytesInDecimalKilobyte, "kB", DataUnit.DecimalKilobyte)];
    static readonly CompositeUnit[] BinaryCompositeUnits = [new (ByteSize.BytesInPebibyte, "PiB", DataUnit.BinaryPebibyte), new (ByteSize.BytesInTebibyte, "TiB", DataUnit.BinaryTebibyte), new (ByteSize.BytesInGibibyte, "GiB", DataUnit.BinaryGibibyte), new (ByteSize.BytesInMebibyte, "MiB", DataUnit.BinaryMebibyte), new (ByteSize.BytesInKibibyte, "KiB", DataUnit.BinaryKibibyte)];
    class CompositeUnit(long Bytes, string Symbol, DataUnit DataUnit)
    {
    }
    public static ByteRate Per(this ByteSize size, TimeSpan interval) =>
        new (size, interval);
}
