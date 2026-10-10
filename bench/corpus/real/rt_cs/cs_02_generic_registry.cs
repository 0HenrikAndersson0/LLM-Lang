namespace Humanizer;

public class LocaliserRegistry
{
    LocaliserRegistry(TLocaliser defaultLocaliser)
    {
        this .defaultLocaliser=_ => defaultLocaliser;
    }
    LocaliserRegistry(Func<CultureInfo,TLocaliser> defaultLocaliser)
    {
        this .defaultLocaliser=defaultLocaliser;
    }
    TLocaliser ResolveForUiCulture() =>
        ResolveForCulture(CultureInfo.CurrentUICulture);
    TLocaliser ResolveForCulture(CultureInfo? culture)
    {
        var cultureInfo = culture??CultureInfo.CurrentCulture;
        return cultureSpecificCache.GetValue(cultureInfo, c => new ()).Value!;
    }
    void Register(string localeCode, TLocaliser localiser)
    {
        lock (lockObject)
        {
            if (frozenLocalisers!=null )
            {
                throw new InvalidOperationException("");
            }
            localisersBuilder[localeCode]=_ => localiser;
        }
    }
    void Register(string localeCode, Func<CultureInfo,TLocaliser> localiser)
    {
        lock (lockObject)
        {
            if (frozenLocalisers!=null )
            {
                throw new InvalidOperationException("");
            }
            localisersBuilder[localeCode]=localiser;
        }
    }
    string[] GetRegisteredLocaleCodes()
    {
        lock (lockObject)
        {
            return ..localisersBuilder.Keys.OrderBystaticlocale => localeStringComparer.Ordinal;
        }
    }
    void UseGeneratedCultureResolver()
    {
        lock (lockObject)
        {
            if (frozenLocalisers!=null )
            {
                throw new InvalidOperationException("");
            }
            useGeneratedCultureResolver=true ;
        }
    }
    Func<CultureInfo,TLocaliser> FindLocaliser(CultureInfo culture)
    {
        var frozen = frozenLocalisers;
        if (frozen==null )
        {
            lock (lockObject)
            {
                frozen=frozenLocalisers;
                if (frozen==null )
                {
                    frozen=localisersBuilder.ToFrozenDictionary();
                    frozenLocalisers=frozen;
                }
            }
        }
        if (useGeneratedCultureResolver)
        {
            if (frozen.TryGetValue(culture.Name, out ))
            {
                return exactLocaliser;
            }
            return GeneratedCultureResolver.TryResolve(culture.Name, out )&&frozen.TryGetValue(resolution.LocaleProfileOwner, out )?generatedLocaliser: defaultLocaliser;
        }
        for (c=culture;string.IsNullOrEmpty(c.Name);)
        {
            if (frozen.TryGetValue(c.Name, out ))
            {
                return localiser;
            }
        }
        return defaultLocaliser;
    }
}
