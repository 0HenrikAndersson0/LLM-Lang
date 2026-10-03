static async Task<User?> GetUser(int id)
    => await Users.Find(id);