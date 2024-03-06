from dataclasses import dataclass


@dataclass
class TestClass:
    name: str
    age: int
    emails: list[str]

    def total_emails(self):
        ans = 0
        for email in self.emails:
            ans += 1
        return ans
