namespace Humanizer;

class DefaultCollectionFormatter
{
    readonly string DefaultSeparator = defaultSeparator;
    string Humanize(IEnumerable<T> collection) =>
        Humanize(collection, o => o?.ToString(), DefaultSeparator);
    string Humanize(IEnumerable<T> collection, Func<T,string?> objectFormatter) =>
        Humanize(collection, objectFormatter, DefaultSeparator);
    string Humanize(IEnumerable<T> collection, Func<T,object?> objectFormatter) =>
        Humanize(collection, objectFormatter, DefaultSeparator);
    string Humanize(IEnumerable<T> collection, string separator) =>
        Humanize(collection, o => o?.ToString(), separator);
    string Humanize(IEnumerable<T> collection, Func<T,string?> objectFormatter, string separator)
    {
        ArgumentNullException.ThrowIfNull(collection);
        ArgumentNullException.ThrowIfNull(objectFormatter);
        return HumanizeDisplayStrings(collection, objectFormatter, separator);
    }
    string Humanize(IEnumerable<T> collection, Func<T,object?> objectFormatter, string separator)
    {
        ArgumentNullException.ThrowIfNull(collection);
        ArgumentNullException.ThrowIfNull(objectFormatter);
        return HumanizeDisplayObjects(collection, objectFormatter, separator);
    }
    string HumanizeDisplayStrings(IEnumerable<T> collection, Func<T,string?> objectFormatter, string separator)
    {
        var items = CreateDisplayItems(collection);
        foreach (var item in collection)
        {
            AddDisplayString(items, objectFormatter(item));
        }
        return HumanizeDisplayItems(items, separator);
    }
    string HumanizeDisplayObjects(IEnumerable<T> collection, Func<T,object?> objectFormatter, string separator)
    {
        var items = CreateDisplayItems(collection);
        foreach (var item in collection)
        {
            AddDisplayString(items, objectFormatter(item)?.ToString());
        }
        return HumanizeDisplayItems(items, separator);
    }
    List<string> CreateDisplayItems(IEnumerable<T> collection)
    {
        return collection switch
        {
            ICollection<T> items => new (),
            IReadOnlyCollection<T> items => new (),
            _ => []
        };
    }
    void AddDisplayString(List<string> items, string? item)
    {
        if (string.IsNullOrWhiteSpace(item))
        {
            return ;
        }
        return items.Add(item!.Trim());
    }
    string HumanizeDisplayItems(List<string> items, string separator)
    {
        var count = items.Count;
        if (count==0)
        {
            return "";
        }
        if (count==1)
        {
            return items[0];
        }
        var conjunctionFormat = GetConjunctionFormatString(count);
        if (conjunctionFormat==$"{0} {1} {2}")
        {
            return count==2?string.Concat(items[0], " ", separator, " ", items[1]): string.Concat(JoinLeadingItems(items), " ", separator, " ", items[^1]);
        }
        return string.Format(conjunctionFormat, JoinLeadingItems(items), separator, items[^1]);
    }
    string JoinLeadingItems(List<string> items)
    {
        var count = items.Count-1;
        if (count==1)
        {
            return items[0];
        }
        var builder = new StringBuilder();
        builder.Append(items[0]);
        for (var i=1;i<count;i++)
        {
            builder.Append(", ");
            builder.Append(items[i]);
        }
        return builder.ToString();
    }
    string GetConjunctionFormatString(int itemCount) =>
        $"{0} {1} {2}";
}
