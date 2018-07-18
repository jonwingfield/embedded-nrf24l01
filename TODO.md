## For Reciever Config

-  You can't really set addresses as all 5 bytes. Only the LSB applies for P2-P5, the other 4 bytes have to be the same.
-  do some checks here and warn for the following (can lead to higher failures):
       * Bits shift only once
       * Bits modulate (hi-lo toggling)
   See https://www.sparkfun.com/datasheets/Components/SMD/nRF24L01Pluss_Preliminary_Product_Specification_v1_0.pdf page 27

-

## Appendex: General Ideas

- Receive Power Detector (RPD) allows you to determine if a good signal is present
- Standby-II transmits quicker but uses much more power (maybe good for the controller node)

