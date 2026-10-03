static string Greet(User u)
    => $"Hello, {u.Name}! You have {u.PendingCount} tasks.";