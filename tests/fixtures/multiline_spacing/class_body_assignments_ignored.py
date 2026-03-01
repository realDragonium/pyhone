class Status(Enum):
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    DONE = "done"


class MyModel(BaseModel):
    name: str
    age: int
    tags: list[str] = []
