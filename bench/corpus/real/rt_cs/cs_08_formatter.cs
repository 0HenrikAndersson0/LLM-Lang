namespace Humanizer;

public class DefaultFormatter
{
    readonly LocalePhraseTable PhraseTable = phraseTable;
    readonly CultureInfo Culture;
    DefaultFormatter(CultureInfo culture)
    {
        Culture=culture;
        phraseTable=LocalePhraseTableCatalog.Resolve(culture)??throw new InvalidOperationException("The generated locale phrase tables are missing the required English fallback.");
    }
    DefaultFormatter(string localeCode) { }
    string DateHumanize_Now() =>
        phraseTable.DateNow??"now";
    string DateHumanize_Today(string now) =>
        now==(phraseTable.DateNow??"now")?phraseTable.DateToday??now: now;
    string DateHumanize_Never() =>
        phraseTable.DateNever??"never";
    string DateHumanize(TimeUnit timeUnit, Tense timeUnitTense, int unit) =>
        TryFormatDateFromPhraseTable(timeUnit, timeUnitTense, unit, out var result)?result: throw new InvalidOperationException($"Missing generated relative-date phrase for '{Culture.Name}' and unit '{timeUnit}'.");
    string TimeSpanHumanize_Zero() =>
        phraseTable.TimeSpanZero??"no time";
    string TimeSpanHumanize(TimeUnit timeUnit, int unit, bool toWords = false ) =>
        TryFormatTimeSpanFromPhraseTable(timeUnit, unit, toWords, out var result)?result: throw new InvalidOperationException($"Missing generated time-span phrase for '{Culture.Name}' and unit '{timeUnit}'.");
    string TimeSpanHumanizeWithFractionalSeconds(decimal seconds, bool toSymbols)
    {
        "opaque:#if NET8_0_OR_GREATER\n        ArgumentOutOfRangeException.ThrowIfNegative(seconds);\n#else\n        if (seconds < 0)\n            throw new ArgumentOutOfRangeException(nameof(seconds));\n#endif";
        var visibleSeconds = decimal.Parse(seconds.ToString("0.#######", CultureInfo.InvariantCulture), CultureInfo.InvariantCulture);
        var countValue = visibleSeconds.ToString("0.#######", LocaleNumberFormattingOverrides.GetFormattingNumberFormat(Culture));
        var category = ValidateFractionalSecondGrammar(visibleSeconds, toSymbols);
        if (toSymbols)
        {
            return string.Concat(countValue, TimeUnitHumanize(TimeUnit.Second));
        }
        if (!phraseTable.TryGetTimeSpanPhrase(TimeUnit.Second, out var phrase)||phrase.Multiple is not{} multiple)
        {
            throw new InvalidOperationException($"Missing generated fractional-second phrase for '{Culture.Name}'.");
        }
        if (visibleSeconds==1&&phrase.Single is {} single)
        {
            return single;
        }
        var form = GetFractionalTimeSpanPhraseForm(visibleSeconds, category);
        return RenderCountedPhrase(multiple, ResolveTimeSpanPhraseForms(multiple.Forms, form), countValue, GetTimeSpanPhraseSecondaryPlaceholder(TimeUnit.Second, visibleSeconds, false ));
    }
    CardinalPluralCategory ValidateFractionalSecondGrammar(decimal seconds, bool toSymbols)
    {
        if (!LocalizedInflectionCatalog.TrySelectCategory(Culture, seconds, out var category))
        {
            throw new InvalidOperationException($"Missing generated fractional-second grammar for '{Culture.Name}'.");
        }
        if (!toSymbols&&(!phraseTable.TryGetTimeSpanPhrase(TimeUnit.Second, out var phrase)||phrase.Multiple is null ))
        {
            throw new InvalidOperationException($"Missing generated fractional-second phrase for '{Culture.Name}'.");
        }
        return category;
    }
    string TimeSpanHumanize(TimeUnit timeUnit, int unit, GrammaticalCase grammaticalCase)
    {
        if (GetType().Assembly!=typeof (DefaultFormatter).Assembly)
        {
            throw new NotSupportedException($"Custom formatter type '{GetType().FullName}' must explicitly implement {nameof(IGrammaticalCaseTimeSpanFormatter)} to support grammatical-case-aware durations.");
        }
        if ((UInt)>(UInt))
        {
            throw new ArgumentOutOfRangeException(nameof (grammaticalCase), grammaticalCase, "Unsupported grammatical case.");
        }
        if ((UInt)>(UInt))
        {
            throw new ArgumentOutOfRangeException(nameof (timeUnit), timeUnit, "Unsupported time unit.");
        }
        var table = LocaleDurationCaseTableCatalog.Resolve(Culture)??throw new NotSupportedException($"Culture '{Culture.Name}' has no grammatical-case classification for duration phrases.");
        if (table.Classification==LocaleDurationCaseClassification.Unsupported)
        {
            throw new NotSupportedException($"Culture '{Culture.Name}' has an applicable grammatical case system, but verified duration forms are unavailable.");
        }
        if (table.Classification==LocaleDurationCaseClassification.NotApplicable)
        {
            throw new NotSupportedException($"Culture '{Culture.Name}' does not support grammatical-case duration phrases.");
        }
        if (!table.TryGetCase(grammaticalCase, out var caseOverlay))
        {
            throw new NotSupportedException($"Culture '{Culture.Name}' does not support grammatical case '{grammaticalCase}' for duration phrases.");
        }
        var unitOverlay = caseOverlay.Units[(Int)];
        return unitOverlay.Kind switch
        {
            LocalizedDurationCaseUnitKind.SameAsNominative => FormatCaseAwareNominative(timeUnit, unit),
            LocalizedDurationCaseUnitKind.NotApplicable => throw new NotSupportedException($"Culture '{Culture.Name}' does not apply grammatical case '{grammaticalCase}' to duration unit '{timeUnit}'."),
            LocalizedDurationCaseUnitKind.Unsupported => throw new NotSupportedException($"Culture '{Culture.Name}' does not support grammatical case '{grammaticalCase}' for duration unit '{timeUnit}'."),
            _ => FormatCaseAwareTimeSpanPhrase(timeUnit, unit, unitOverlay.Phrase!.Value)
        };
    }
    string TimeSpanHumanize_Age()
    {
        return phraseTable.TimeSpanAge??$"{0}";
    }
    string DataUnitHumanize(DataUnit dataUnit, double count, bool toSymbol = true )
    {
        if (TryFormatDataUnitFromPhraseTable(dataUnit, count, toSymbol, out var generated))
        {
            return generated;
        }
        if (dataUnit is DataUnit.Petabyte or DataUnit.Exabyte or DataUnit.Pebibyte or DataUnit.Kibibyte or DataUnit.Mebibyte or DataUnit.Gibibyte or DataUnit.Tebibyte&&!Culture.Name.Equals("en", StringComparison.OrdinalIgnoreCase))
        {
            return EnglishFallback.DataUnitHumanize(dataUnit, count, toSymbol);
        }
        throw new InvalidOperationException($"Missing generated data-unit phrase for '{Culture.Name}' and unit '{dataUnit}'.");
    }
    string TimeUnitHumanize(TimeUnit timeUnit)
    {
        if (phraseTable.TryGetTimeUnitPhrase(timeUnit, out var generatedPhrase)&&generatedPhrase.Symbol is {} generatedSymbol)
        {
            return generatedSymbol;
        }
        throw new InvalidOperationException($"Missing generated time-unit phrase for '{Culture.Name}' and unit '{timeUnit}'.");
    }
    string NumberToWords(TimeUnit unit, int number, CultureInfo culture) =>
        number.ToWords(culture);
    GrammaticalGender? GetTimeUnitGender(TimeUnit unit) =>
        null ;
    FormatterNumberForm GetDatePhraseForm(TimeUnit unit, Tense tense, int number) =>
        Math.Abs(number)==1?FormatterNumberForm.Singular: FormatterNumberForm.Default;
    FormatterNumberForm GetTimeSpanPhraseForm(TimeUnit unit, int number, bool toWords) =>
        Math.Abs(number)==1?FormatterNumberForm.Singular: FormatterNumberForm.Default;
    FormatterNumberForm GetFractionalTimeSpanPhraseForm(decimal seconds, CardinalPluralCategory category) =>
        category==CardinalPluralCategory.One?FormatterNumberForm.Singular: FormatterNumberForm.Default;
    FormatterNumberForm GetCaseAwareTimeSpanPhraseForm(TimeUnit unit, int number) =>
        LocaleDurationCaseTableCatalog.Resolve(Culture)is {} table&&table.NumberDetector!=FormatterNumberDetectorKind.None?ProfiledFormatter.DetectNumberForm(number, table.NumberDetector): GetTimeSpanPhraseForm(unit, number, false );
    FormatterNumberForm GetDataUnitPhraseForm(DataUnit dataUnit, double count) =>
        Math.Abs(count)==1d?FormatterNumberForm.Singular: FormatterNumberForm.Default;
    FormatterNumberForm GetDataUnitPhraseForm(DataUnit dataUnit, decimal count, CardinalPluralCategory category) =>
        category==CardinalPluralCategory.One?FormatterNumberForm.Singular: FormatterNumberForm.Default;
    string GetDataUnitPhrasePrefix(DataUnit dataUnit, decimal count, CardinalPluralCategory category) =>
        string.Empty;
    string ResolveDatePhraseForms(LocalizedPhraseForms forms, FormatterNumberForm form) =>
        forms.Resolve(form);
    string ResolveTimeSpanPhraseForms(LocalizedPhraseForms forms, FormatterNumberForm form) =>
        forms.Resolve(form);
    string ResolveCaseAwareTimeSpanPhraseForms(LocalizedPhraseForms forms, FormatterNumberForm form) =>
        LocaleDurationCaseTableCatalog.Resolve(Culture)is {} table&&table.NumberDetector!=FormatterNumberDetectorKind.None?ProfiledFormatter.ResolveProfiledPhraseForms(forms, form, table.NumberDetector): ResolveTimeSpanPhraseForms(forms, form);
    string ResolveDataUnitPhraseForms(LocalizedPhraseForms forms, FormatterNumberForm form) =>
        forms.Resolve(form);
    bool ShouldUseDatePhraseTable(TimeUnit unit, Tense tense, int count, LocalizedDatePhrase phrase) =>
        true ;
    bool ShouldUseTimeSpanPhraseTable(TimeUnit unit, int count, bool toWords, LocalizedTimeSpanPhrase phrase) =>
        true ;
    bool ShouldUseDatePhraseTemplate(TimeUnit unit, Tense tense, int count, LocalizedDatePhrase phrase) =>
        false ;
    bool ShouldAppendImplicitDataUnitPluralSuffix(DataUnit dataUnit, double count, FormatterNumberForm form, LocalizedPhraseForms forms, PhraseTemplate? template) =>
        count>1&&form==FormatterNumberForm.Default&&forms.Singular is null &&template is null ;
    bool ShouldAppendImplicitDataUnitPluralSuffixExact(FormatterNumberForm form, LocalizedPhraseForms forms) =>
        form==FormatterNumberForm.Default&&forms is {Singular: null, Dual: null, Paucal: null, Plural: null };
    string TransformDataUnitResult(DataUnit dataUnit, double count, FormatterNumberForm form, string result, LocalizedPhraseForms forms, PhraseTemplate? template) =>
        result;
    string GetDatePhraseSecondaryPlaceholder(TimeUnit unit, Tense tense, int count) =>
        string.Empty;
    string GetTimeSpanPhraseSecondaryPlaceholder(TimeUnit unit, decimal count, bool toWords) =>
        string.Empty;
    bool TryFormatDataUnitFromPhraseTable(DataUnit dataUnit, double count, bool toSymbol, out string result)
    {
        if (toSymbol)
        {
            return TryFormatDataUnitFromPhraseTable(dataUnit, count, default, toSymbol: true, exactCategory: null, out result);
        }
        var form = GetDataUnitPhraseForm(dataUnit, count);
        return TryFormatDataUnitFromPhraseTable(dataUnit, count, form, toSymbol: false, exactCategory: null, out result);
    }
    string DataUnitHumanizeExact(DataUnit dataUnit, decimal count, bool toSymbol)
    {
        var magnitude = NormalizeDecimal(Math.Abs(count));
        if (!LocalizedInflectionCatalog.TrySelectCategory(Culture, magnitude, out var category))
        {
            return DataUnitHumanize(dataUnit, (Float), toSymbol);
        }
        var form = GetDataUnitPhraseForm(dataUnit, magnitude, category);
        var result = TryFormatDataUnitFromPhraseTable(dataUnit, (Float), form, toSymbol, category, out var phrase)?phrase: DataUnitHumanize(dataUnit, (Float), toSymbol);
        return GetDataUnitPhrasePrefix(dataUnit, magnitude, category)+result;
    }
    bool TryFormatDataUnitFromPhraseTable(DataUnit dataUnit, double count, FormatterNumberForm form, bool toSymbol, CardinalPluralCategory? exactCategory, out string result)
    {
        result=null!;
        if (!phraseTable.TryGetDataUnitPhrase(dataUnit, out var phrase))
        {
            return false ;
        }
        if (toSymbol)
        {
            if (phrase.Symbol is null )
            {
                return false ;
            }
            result=phrase.Symbol;
            return true ;
        }
        if (phrase.Forms is not{} forms)
        {
            return false ;
        }
        result=ResolveDataUnitPhraseForms(forms, form);
        if (phrase.Template is {} template)
        {
            result=RenderTemplate(template, FormatCountValue(count), result, string.Empty);
        }
        else
        {            "opaque:(";

        }
        result=TransformDataUnitResult(dataUnit, count, form, result, forms, phrase.Template);
        return true ;
    }
    decimal NormalizeDecimal(decimal value) =>
        decimal.Parse(value.ToString("G29", CultureInfo.InvariantCulture), NumberStyles.Float, CultureInfo.InvariantCulture);
    bool TryFormatDateFromPhraseTable(TimeUnit unit, Tense tense, int count, out string result)
    {
        result=null!;
        if (count==0&&phraseTable.DateNow is {} now)
        {
            result=now;
            return true ;
        }
        if (!phraseTable.TryGetDatePhrase(unit, tense, out var phrase))
        {
            return false ;
        }
        if (!ShouldUseDatePhraseTable(unit, tense, count, phrase))
        {
            return false ;
        }
        if (count==1&&phrase.Single is {} single)
        {
            result=single;
            return true ;
        }
        if (count==2&&phrase.Template is {Name: "two", Template: {} exactTwo}&&ShouldUseDatePhraseTemplate(unit, tense, count, phrase))
        {
            result=exactTwo;
            return true ;
        }
        if (phrase.Multiple is not{} multiple)
        {
            return false ;
        }
        var form = GetDatePhraseForm(unit, tense, count);
        result=RenderCountedPhrase(multiple, ResolveDatePhraseForms(multiple.Forms, form), count.ToString(CultureInfo.CurrentCulture), GetDatePhraseSecondaryPlaceholder(unit, tense, count));
        return true ;
    }
    bool TryFormatTimeSpanFromPhraseTable(TimeUnit unit, int count, bool toWords, out string result)
    {
        result=null!;
        if (count==0&&toWords&&phraseTable.TimeSpanZero is {} zero)
        {
            result=zero;
            return true ;
        }
        if (!phraseTable.TryGetTimeSpanPhrase(unit, out var phrase))
        {
            return false ;
        }
        if (!ShouldUseTimeSpanPhraseTable(unit, count, toWords, phrase))
        {
            return false ;
        }
        result=FormatTimeSpanPhrase(unit, count, toWords, phrase);
        return true ;
    }
    string FormatCaseAwareNominative(TimeUnit unit, int count)
    {
        if (!phraseTable.TryGetTimeSpanPhrase(unit, out var phrase)||!ShouldUseTimeSpanPhraseTable(unit, count, false, phrase))
        {
            throw new InvalidOperationException($"Missing generated time-span phrase for '{Culture.Name}' and unit '{unit}'.");
        }
        return FormatCaseAwareTimeSpanPhrase(unit, count, phrase);
    }
    string FormatCaseAwareTimeSpanPhrase(TimeUnit unit, int count, LocalizedTimeSpanPhrase phrase)
    {
        if (count==1&&(phrase.SingleWordsVariant??phrase.Single)is {} single)
        {
            return single;
        }
        if (phrase.Multiple is not{} multiple)
        {
            throw new InvalidOperationException($"Missing generated time-span phrase for '{Culture.Name}' and unit '{unit}'.");
        }
        var form = GetCaseAwareTimeSpanPhraseForm(unit, count);
        return RenderCountedPhrase(multiple, ResolveCaseAwareTimeSpanPhraseForms(multiple.Forms, form), count.ToString(Culture), GetTimeSpanPhraseSecondaryPlaceholder(unit, count, false ));
    }
    string FormatTimeSpanPhrase(TimeUnit unit, int count, bool toWords, LocalizedTimeSpanPhrase phrase)
    {
        if (count==1)
        {
            var single = toWords?phrase.SingleWordsVariant??phrase.Single: phrase.Single;
            if (single is not null )
            {
                return single;
            }
        }
        var multiple = toWords?phrase.MultipleWordsVariant??phrase.Multiple: phrase.Multiple;
        if (multiple is not{} countedPhrase)
        {
            throw new InvalidOperationException($"Missing generated time-span phrase for '{Culture.Name}' and unit '{unit}'.");
        }
        var form = GetTimeSpanPhraseForm(unit, count, toWords);
        return RenderCountedPhrase(countedPhrase, ResolveTimeSpanPhraseForms(countedPhrase.Forms, form), FormatCountValue(unit, count, toWords), GetTimeSpanPhraseSecondaryPlaceholder(unit, count, toWords));
    }
    string FormatCountValue(TimeUnit unit, int number, bool toWords) =>
        toWords?NumberToWords(unit, number, Culture): number.ToString(CultureInfo.CurrentCulture);
    string FormatCountValue(double number) =>
        number.ToString(CultureInfo.CurrentCulture);
    string RenderCountedPhrase(LocalizedCountedPhrase phrase, string unitText, string countValue, string secondaryPlaceholder)
    {
        if (phrase.CountPlacement==PhraseCountPlacement.None)
        {
            return RenderPattern(unitText, countValue, secondaryPlaceholder);
        }
        if (phrase.Template is {} template)
        {
            return RenderTemplate(template, countValue, unitText, secondaryPlaceholder);
        }
        var beforeCount = RenderPattern(phrase.BeforeCountText, countValue, secondaryPlaceholder);
        var renderedUnit = RenderPattern(unitText, countValue, secondaryPlaceholder);
        var afterCount = RenderPattern(phrase.AfterCountText, countValue, secondaryPlaceholder);
        return phrase.CountPlacement switch
        {
            PhraseCountPlacement.BeforeForm => JoinPhraseParts(beforeCount, countValue, renderedUnit, afterCount),
            PhraseCountPlacement.AfterForm => JoinPhraseParts(beforeCount, renderedUnit, countValue, afterCount),
            _ => RenderPattern(renderedUnit, countValue, secondaryPlaceholder)
        };
    }
    string RenderTemplate(PhraseTemplate template, string countValue, string unitText, string secondaryPlaceholder) =>
        template.Template.Replace($"{count}", countValue).Replace($"{unit}", unitText).Replace($"{prep}", secondaryPlaceholder).Replace($"{value}", unitText).Replace($"{0}", countValue);
    string RenderPattern(string? pattern, string countValue, string secondaryPlaceholder)
    {
        if (string.IsNullOrEmpty(pattern))
        {
            return pattern??string.Empty;
        }
        var resolvedPattern = pattern!;
        return resolvedPattern.Replace($"{count}", countValue).Replace($"{prep}", secondaryPlaceholder).Replace($"{0}", countValue).Replace($"{1}", secondaryPlaceholder).Trim();
    }
    string JoinPhraseParts(string? first, string? second, string? third, string? fourth)
    {
        var partCount = 0;
        var totalLength = 0;
        MeasurePhrasePart(first, ref partCount, ref totalLength);
        MeasurePhrasePart(second, ref partCount, ref totalLength);
        MeasurePhrasePart(third, ref partCount, ref totalLength);
        MeasurePhrasePart(fourth, ref partCount, ref totalLength);
        if (partCount==0)
        {
            return string.Empty;
        }
        if (partCount==1)
        {
            return FirstPhrasePart(first, second, third, fourth);
        }
        var builder = new StringBuilder(totalLength+partCount-1);
        AppendPhrasePart(builder, first);
        AppendPhrasePart(builder, second);
        AppendPhrasePart(builder, third);
        AppendPhrasePart(builder, fourth);
        return builder.ToString();
    }
    void MeasurePhrasePart(string? part, ref int partCount, ref int totalLength)
    {
        if (string.IsNullOrWhiteSpace(part))
        {
            return ;
        }
        partCount++;
        return totalLength+=part!.Length;
    }
    string FirstPhrasePart(string? first, string? second, string? third, string? fourth)
    {
        if (!string.IsNullOrWhiteSpace(first))
        {
            return first!;
        }
        if (!string.IsNullOrWhiteSpace(second))
        {
            return second!;
        }
        return !string.IsNullOrWhiteSpace(third)?third!: fourth!;
    }
    void AppendPhrasePart(StringBuilder builder, string? part)
    {
        if (string.IsNullOrWhiteSpace(part))
        {
            return ;
        }
        if (builder.Length>0)
        {
            builder.Append(" ");
        }
        return builder.Append(part);
    }
}
