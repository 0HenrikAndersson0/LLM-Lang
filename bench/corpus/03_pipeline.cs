static List<Order> ActiveHeavy()
    => Orders
        .Where(o => o.Paid && o.Total > 100m)
        .Select(o => o.Total)
        .ToList();