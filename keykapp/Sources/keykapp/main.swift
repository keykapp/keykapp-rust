import System

let logPath: FilePath = "/tmp/keykapp.log"
let fd = try FileDescriptor.open(logPath, .writeOnly, options: .append)
try fd.closeAfter { try fd.writeAll("hello world\n".utf8) }
