static Response Handle(Request r)
    => r.Kind switch
    {
        "GET"  => Read(r.Path),
        "POST" => Write(r.Body),
        _      => Response.Err(405, "method not allowed"),
    };