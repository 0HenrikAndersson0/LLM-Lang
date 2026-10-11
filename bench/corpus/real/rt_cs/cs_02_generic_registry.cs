namespace Humanizer;
using System.Runtime.CompilerServices;

public class LocaliserRegistry<TLocaliser>
{
    readonly Dictionary<string,Func<CultureInfo,TLocaliser>> localisersBuilder = [];
    readonly object lockObject = new ();
    volatile FrozenDictionary<string,Func<CultureInfo,TLocaliser>>? frozenLocalisers;
    readonly Func<CultureInfo,TLocaliser> defaultLocaliser;
    readonly ConditionalWeakTable<CultureInfo,StrongBox<TLocaliser>> cultureSpecificCache = new ();
    volatile bool useGeneratedCultureResolver;
    public LocaliserRegistry(TLocaliser defaultLocaliser)
    {
        this .defaultLocaliser=_ => defaultLocaliser;
    }
    public LocaliserRegistry(Func<CultureInfo,TLocaliser> defaultLocaliser)
    {
        this .defaultLocaliser=defaultLocaliser;
    }
    public TLocaliser ResolveForUiCulture() =>
        ResolveForCulture(CultureInfo.CurrentUICulture);
    public TLocaliser ResolveForCulture(CultureInfo? culture)
    {
        var cultureInfo = culture??CultureInfo.CurrentCulture;
        return cultureSpecificCache.GetValue(cultureInfo, c => new (FindLocaliser(c)(c))).Value!;
    }
    public void Register(string localeCode, TLocaliser localiser)
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
    public void Register(string localeCode, Func<CultureInfo,TLocaliser> localiser)
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
    internal string[] GetRegisteredLocaleCodes()
    {
        lock (lockObject)
        {
            return [..localisersBuilder.Keys.OrderBy(static locale => locale, StringComparer.Ordinal)];
        }
    }
    internal void UseGeneratedCultureResolver()
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
    internal Func<CultureInfo,TLocaliser> FindLocaliser(CultureInfo culture)
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
        for (var c=culture;!string.IsNullOrEmpty(c.Name);c=c.Parent)
        {
            if (frozen.TryGetValue(c.Name, out var localiser))
            {
                return localiser;
            }
        }
        return defaultLocaliser;
    }
}
