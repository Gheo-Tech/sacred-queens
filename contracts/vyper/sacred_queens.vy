event Transfer:
    sender:   indexed(address)
    receiver: indexed(address)
    value:    uint256

event Stake:
    account:   indexed(address)
    value:    uint256

event Unstake:
    account:   indexed(address)
    value:    uint256

event Approval:
    owner:    indexed(address)
    spender:  indexed(address)
    value:    uint256

name:      public(String[32])
symbol:    public(String[32])
decimals:  public(uint8)

balanceOf:    public(HashMap[address,  uint256])         
stakeOf:      public(HashMap[address,  uint256])         
stakeRewards: public(HashMap[address,  uint256])
allowance:    public(HashMap[address,  HashMap[address,  uint256]])
totalSupply:  public(uint256)                            
minter:       address                                    

@external
def __init__(_name: String[32], _symbol: String[32], _decimals: uint8, _supply: uint256):
    init_supply: uint256 = _supply * 10 ** convert(_decimals, uint256)
    self.name = _name
    self.symbol = _symbol
    self.decimals = _decimals
    self.balanceOf[msg.sender] = init_supply
    self.totalSupply = init_supply
    self.minter = msg.sender
    log Transfer(ZERO_ADDRESS, msg.sender, init_supply)

@external
def transfer(_to : address, _value : uint256) -> bool:
    self.balanceOf[msg.sender] -= _value
    self.balanceOf[_to] += _value
    log Transfer(msg.sender, _to, _value)
    return True

@external
def transferFrom(_from : address, _to : address, _value : uint256) -> bool:
    self.balanceOf[_from] -= _value
    self.balanceOf[_to] += _value
    self.allowance[_from][msg.sender] -= _value
    log Transfer(_from, _to, _value)
    return True

@external
def approve(_spender : address, _value : uint256) -> bool:
    self.allowance[msg.sender][_spender] = _value
    log Approval(msg.sender, _spender, _value)
    return True

@external
def mintRewards(_to: address, _value: uint256):
  assert msg.sender == self.minter
  assert _to != ZERO_ADDRESS
  self.stakeRewards[_to] += _value

@external
def stake(_value: uint256) -> bool:
    self.balanceOf[msg.sender] -= _value
    self.stakeOf[msg.sender] += _value
    log Stake(msg.sender, _value)
    return True

@external
def unstake(_value: uint256) -> bool:
    self.stakeOf[msg.sender] -= _value
    self.balanceOf[msg.sender] += _value
    log Unstake(msg.sender, _value)
    return True
