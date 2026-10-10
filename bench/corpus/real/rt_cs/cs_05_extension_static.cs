namespace Humanizer;

public static class MetricNumeralExtensions
{
    static bool ShouldKeepTrailingZeros(MetricNumeralFormats? formats, int? decimals)
    {
        if (!decimals.HasValue||!formats.HasValue||!formats.Value.HasFlag(MetricNumeralFormats.KeepTrailingZeros))
        {
            return false ;
        }
        if (decimals is <0 or >15)
        {
            throw new ArgumentOutOfRangeException(nameof (decimals), decimals, "Fixed metric precision must be between 0 and 15.");
        }
        return true ;
    }
    static string FormatLongWithTrailingZeros(long input, int decimals, NumberFormatInfo nfi) =>
        decimals>0?input.ToString(nfi)+nfi.NumberDecimalSeparator+new string("0", decimals): input.ToString(nfi);
    static double FromMetric(this string input)
    {
        input=CleanRepresentation(input);
        return BuildNumber(input, input[^1]);
    }
    static string ToMetric(this int input, MetricNumeralFormats? formats = null , int? decimals = null )
    {
        if (!formats.HasValue&&!decimals.HasValue)
        {
            return BuildDefaultRepresentation(input);
        }
        return ((Float)).ToMetric(formats, decimals);
    }
    static string BuildDefaultRepresentation(int input)
    {
        if (input==0)
        {
            return input.ToString();
        }
        var absolute = Math.Abs((Int64));
        var nfi = LocaleNumberFormattingOverrides.GetFormattingNumberFormat(CultureInfo.CurrentCulture);
        if (absolute<1_000)
        {
            return input.ToString(nfi);
        }
        if (absolute<1_000_000)
        {
            return (input/1_000d).ToString("G15", nfi)+"k";
        }
        if (absolute<1_000_000_000)
        {
            return (input/1_000_000d).ToString("G15", nfi)+"M";
        }
        return (input/1_000_000_000d).ToString("G15", nfi)+"G";
    }
    static string ToMetric(this long input, MetricNumeralFormats? formats = null , int? decimals = null )
    {
        if (input.Equals(0))
        {
            if (ShouldKeepTrailingZeros(formats, decimals))
            {
                var nfi = LocaleNumberFormattingOverrides.GetFormattingNumberFormat(CultureInfo.CurrentCulture);
                var space = formats.GetValueOrDefault().HasFlag(MetricNumeralFormats.WithSpace)?" ": string.Empty;
                return FormatLongWithTrailingZeros(input, decimals.GetValueOrDefault(), nfi)+space;
            }
            return input.ToString();
        }
        return BuildRepresentation(input, formats, decimals);
    }
    static string ToMetric(this double input, MetricNumeralFormats? formats = null , int? decimals = null )
    {
        if (input.Equals(0))
        {
            if (ShouldKeepTrailingZeros(formats, decimals))
            {
                var nfi = LocaleNumberFormattingOverrides.GetFormattingNumberFormat(CultureInfo.CurrentCulture);
                var space = formats.GetValueOrDefault().HasFlag(MetricNumeralFormats.WithSpace)?" ": string.Empty;
                return input.ToString($"F{decimals.GetValueOrDefault()}", nfi)+space;
            }
            return input.ToString();
        }
        if (input.IsOutOfRange())
        {
            throw new ArgumentOutOfRangeException(nameof (input));
        }
        return BuildRepresentation(input, formats, decimals);
    }
    static string CleanRepresentation(string input)
    {
        ArgumentNullException.ThrowIfNull(input);
        input=input.Trim();
        input=ReplaceNameBySymbol(input);
        if (input.Length==0||input.IsInvalidMetricNumeral())
        {
            throw new ArgumentException("Empty or invalid Metric string.", nameof (input));
        }
        return input.Replace(" ", string.Empty);
    }
    static double BuildNumber(string input, Char last) =>
        char.IsLetter(last)?BuildMetricNumber(input, last): double.Parse(input);
    static double BuildMetricNumber(string input, Char last)
    {
        var number = double.Parse(input[..^1]);
        var indexInPositive = Symbols[0].IndexOf(last);
        if (indexInPositive>=0)
        {
            var exponent = (indexInPositive+1)*3.0;
            return number*Math.Pow(10, exponent);
        }
        else
        {            var indexInNegative = Symbols[1].IndexOf(last);

        }
    }
    static string ReplaceNameBySymbol(string input) =>
        UnitPrefixes.Aggregate(input, current => current.Replace(unitPrefix.Value.Name, unitPrefix.Key.ToString()));
    static string BuildRepresentation(long input, MetricNumeralFormats? formats, int? decimals)
    {
        var number = Math.Abs(input/10);
        var exponent = 0;
        while (number>0)
        {
            exponent++;
            number/=10;
        }
        var scale = exponent/3;
        if (!scale.Equals(0))
        {
            return BuildMetricRepresentation(input, scale, formats, decimals);
        }
        var nfi = LocaleNumberFormattingOverrides.GetFormattingNumberFormat(CultureInfo.CurrentCulture);
        var representation = ShouldKeepTrailingZeros(formats, decimals)?FormatLongWithTrailingZeros(input, decimals.GetValueOrDefault(), nfi): input.ToString(nfi);
        var space = (formats&MetricNumeralFormats.WithSpace)==MetricNumeralFormats.WithSpace?" ": string.Empty;
        return representation+space;
    }
    static string BuildMetricRepresentation(long input, int scale, MetricNumeralFormats? formats, int? decimals)
    {
        var exponent = scale*3;
        var divisor = 1L;
        for (var i=0;i<scale;i++)
        {
            divisor*=1000;
        }
        var number = input/divisor;
        var fractionalPart = Math.Abs(input%divisor);
        if (Math.Abs(number)>=1000&&exponent<Symbols[0].Count*3)
        {
            exponent+=3;
            scale++;
            divisor*=1000;
            number=input/divisor;
            fractionalPart=Math.Abs(input%divisor);
        }
        var requestedDecimals = decimals;
        var keepTrailingZeros = ShouldKeepTrailingZeros(formats, decimals);
        if (!decimals.HasValue)
        {
            decimals=exponent;
        }
        var fractionalPartCharacters = Array.Empty<char>();
        if (decimals>0)
        {
            fractionalPartCharacters=fractionalPart.ToString().PadLeft(exponent, "0").ToCharArray();
            if (!decimals.HasValue||(decimals>=fractionalPartCharacters.Length))
            {
                decimals=fractionalPartCharacters.Length;
            }
            else
            {                "opaque:(";

            }
            while ((decimals>0)&&(fractionalPartCharacters[decimals.Value-1]=="0"))
            {
                decimals--;
            }
        }
        var nfi = LocaleNumberFormattingOverrides.GetFormattingNumberFormat(CultureInfo.CurrentCulture);
        if (decimals==0)
        {
            var roundingPoint = divisor/2;
            if (divisor>1)
            {
                if (fractionalPart>roundingPoint)
                {
                    number+=Math.Sign(number);
                }
                else
                {                    "opaque:(";

                }
                if (Math.Abs(number)==1000)
                {
                    number/=1000;
                    scale++;
                }
            }
            var space = formats.HasValue&&formats.Value.HasFlag(MetricNumeralFormats.WithSpace)?" ": string.Empty;
            var representation = keepTrailingZeros?FormatLongWithTrailingZeros(number, requestedDecimals.GetValueOrDefault(), nfi): number.ToString(nfi);
            var displayedNumber = decimal.TryParse(representation, NumberStyles.Float, nfi, out var parsedNumber)?parsedNumber: number;
            var unitText = GetUnitText(scale, formats, displayedNumber);
            return representation+space+unitText;
        }
        else
        {            var space = formats.HasValue&&formats.Value.HasFlag(MetricNumeralFormats.WithSpace)?" ": string.Empty;

        }
    }
    static string BuildRepresentation(double input, MetricNumeralFormats? formats, int? decimals)
    {
        var exponent = (Int);
        if (!exponent.Equals(0))
        {
            return BuildMetricRepresentation(input, exponent, formats, decimals);
        }
        if (decimals.HasValue)
        {
            input=Math.Round(input, decimals.Value);
            if (Math.Abs(input)>=1000)
            {
                return BuildMetricRepresentation(input, 1, formats, decimals);
            }
        }
        var nfi = LocaleNumberFormattingOverrides.GetFormattingNumberFormat(CultureInfo.CurrentCulture);
        var representation = ShouldKeepTrailingZeros(formats, decimals)?input.ToString($"F{decimals.GetValueOrDefault()}", nfi): input.ToString(nfi);
        var space = (formats&MetricNumeralFormats.WithSpace)==MetricNumeralFormats.WithSpace?" ": string.Empty;
        return representation+space;
    }
    static string BuildMetricRepresentation(double input, int exponent, MetricNumeralFormats? formats, int? decimals)
    {
        var number = input*Math.Pow(1000, -exponent);
        if (decimals.HasValue)
        {
            number=Math.Round(number, decimals.Value);
        }
        if (Math.Abs(number)>=1000&&exponent<Symbols[0].Count)
        {
            number/=1000;
            exponent+=1;
        }
        var nfi = LocaleNumberFormattingOverrides.GetFormattingNumberFormat(CultureInfo.CurrentCulture);
        var representation = ShouldKeepTrailingZeros(formats, decimals)?number.ToString($"F{decimals.GetValueOrDefault()}", nfi): number.ToString("G15", nfi);
        var displayedNumber = decimal.TryParse(representation, NumberStyles.Float, nfi, out var parsedNumber)?parsedNumber: (Decimal);
        var unitText = GetUnitText(exponent, formats, displayedNumber);
        var space = formats.HasValue&&formats.Value.HasFlag(MetricNumeralFormats.WithSpace)?" ": string.Empty;
        return representation+space+unitText;
    }
    static string GetUnitText(int scale, MetricNumeralFormats? formats, decimal displayedNumber)
    {
        return Math.Sign(scale) switch
        {
            +1 => GetUnitText(Symbols[0][scale-1], formats, displayedNumber),
            -1 => GetUnitText(Symbols[1][-scale-1], formats, displayedNumber),
            _ => string.Empty
        };
    }
    static string GetUnitText(Char symbol, MetricNumeralFormats? formats, decimal displayedNumber)
    {
        if (formats.HasValue)
        {
            var formatValue = formats.Value;
            if (formatValue.HasFlag(MetricNumeralFormats.UseName))
            {
                return UnitPrefixes[symbol].Name;
            }
            if (formatValue.HasFlag(MetricNumeralFormats.UseShortScaleWord))
            {
                return UnitPrefixes[symbol].ShortScaleWord;
            }
            if (formatValue.HasFlag(MetricNumeralFormats.UseLongScaleWord))
            {
                return UnitPrefixes[symbol].LongScaleWord;
            }
            if (formatValue.HasFlag(MetricNumeralFormats.UseScaleWord))
            {
                return LocalizedMetricScaleWordCatalog.TryResolve(CultureInfo.CurrentUICulture, symbol, displayedNumber, out var scaleWord)?scaleWord: symbol.ToString();
            }
        }
        return symbol.ToString();
    }
    static bool IsOutOfRange(this double input)
    {
        void outside(double min, double max)
    {
        !(max>input&&input>min);
    }
        return (Math.Sign(input)==1&&outside(SmallLimit, BigLimit))||(Math.Sign(input)==-1&&outside(-BigLimit, -SmallLimit));
    }
    static bool IsInvalidMetricNumeral(this string input)
    {
        var index = input.Length-1;
        var last = input[index];
        var isSymbol = UnitPrefixes.ContainsKey(last);
        return !double.TryParse(isSymbol?input[..index]: input, out _);
    }
    class UnitPrefix
    {
        readonly string Name = name;
        readonly string ShortScaleWord = shortScaleWord;
        readonly string LongScaleWord = longScaleWord??ShortScaleWord;
    }
}
