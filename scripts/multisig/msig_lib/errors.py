class MsigError(Exception):
    exit_code = 1


class UsageError(MsigError):
    exit_code = 1


class CheckFailed(MsigError):
    exit_code = 2


class CliError(MsigError):
    exit_code = 3


class ChainError(MsigError):
    exit_code = 4
