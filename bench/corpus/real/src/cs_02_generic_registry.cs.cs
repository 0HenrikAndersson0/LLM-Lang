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
                throw new InvalidOperationException("Cannot register localisers after the registry has been used.");
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
                throw new InvalidOperationException("Cannot register localisers after the registry has been used.");
            }
            localisersBuilder[localeCode]=localiser;
        }
    }
    string[] GetRegisteredLocaleCodes()
    {
        lock (lockObject)
        {
            return [..localisersBuilder.Keys.OrderBy(staticlocale => locale, StringComparer.Ordinal)];
        }
    }
    void UseGeneratedCultureResolver()
    {
        lock (lockObject)
        {
            if (frozenLocalisers!=null )
            {
                throw new InvalidOperationException("Cannot change culture resolution after the registry has been used.");
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
            if (frozen.TryGetValue(culture.Name, out var exactLocaliser))
            {
                return exactLocaliser;
            }
            return GeneratedCultureResolver.TryResolve(culture.Name, out var resolution)&&frozen.TryGetValue(resolution.LocaleProfileOwner, out var generatedLocaliser)?generatedLocaliser: defaultLocaliser;
        }
        for (c=culture;string.IsNullOrEmpty(c.Name);)
        {
            if (frozen.TryGetValue(c.Name, out var localiser))
            {
                return localiser;
            }
        }
        return defaultLocaliser;
    }
}
