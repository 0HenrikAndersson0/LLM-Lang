static async Task<User?> Load(int id)
{
    try { return await Repo.Get(id); }
    catch (NotFound) { return null; }
    catch (Error e) { throw e; }
    finally { Log("done"); }
}