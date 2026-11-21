import Foundation
import RemindersCore

setvbuf(stdout, nil, _IOLBF, 0)
let args = Array(CommandLine.arguments.dropFirst())
let code = await run(args, store: EventKitStore()) { print($0) }
if code == 0, args.first == "watch" {
    // Runs until the parent closes stdin.
    while readLine() != nil {}
}
exit(code)
