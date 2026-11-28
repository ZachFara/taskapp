-- Deferred issues (Process Inbox "Defer"): views hide an issue until this day. NULL means never hidden.
ALTER TABLE issues ADD COLUMN hidden_until TEXT;
