import Foundation

public struct ReminderList: Equatable {
    public var id: String
    public var title: String
    public init(id: String, title: String) { self.id = id; self.title = title }
}

/// A reminder as the store reports it, before mapping to the JSON contract.
public struct RawReminder {
    public var externalId: String
    public var title: String
    public var notes: String?
    public var isCompleted: Bool
    /// Date-only when `hour` is nil; `timeZone` nil means the device zone.
    public var due: DateComponents?
    public init(externalId: String, title: String, notes: String? = nil, isCompleted: Bool = false, due: DateComponents? = nil) {
        self.externalId = externalId; self.title = title; self.notes = notes
        self.isCompleted = isCompleted; self.due = due
    }
}

public struct HelperError: Error, Equatable {
    public var message: String
    public init(_ message: String) { self.message = message }
}

/// EventKit stays behind this so the JSON mapping can be tested with a fake.
public protocol ReminderStore {
    func requestAccess() async throws -> Bool
    func lists() throws -> [ReminderList]
    /// Throws `HelperError` for an unknown list id.
    func reminders(inList id: String) async throws -> [RawReminder]
    /// Returns false when the list has no reminder with that external id; other lists are never touched.
    func delete(externalId: String, inList id: String) throws -> Bool
    func onChange(_ handler: @escaping () -> Void)
}
